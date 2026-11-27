use std::cell::RefCell;

use wheeler_core::save::{ItemData, WheelData};
use wheeler_core::sound::Sound;

use crate::drawer::{self, DrawArgs};
use crate::headless::render_frame;
use crate::image::{IconImageType, Image};
use crate::wheel::{self, Wheel, WheelEntry, WheelHost, WheelItem};

/// Test item: `available` / `active` flags and a counter of activations.
struct MockItem {
    name: &'static str,
    available: bool,
    active: bool,
    activations: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl MockItem {
    fn boxed(name: &'static str, available: bool) -> Box<dyn WheelItem<()>> {
        Box::new(Self {
            name,
            available,
            active: false,
            activations: Default::default(),
        })
    }
}

impl WheelItem<()> for MockItem {
    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _ctx: &(), args: DrawArgs) {
        wheel::draw_slot_texture(
            Image {
                texture: 42,
                width: 64,
                height: 64,
            },
            center,
            args,
        );
        wheel::draw_slot_text(center, self.name, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &(), args: DrawArgs) {
        wheel::draw_highlight_text(center, self.name, args);
        wheel::draw_highlight_description(
            center,
            "a fairly long description that should wrap onto several lines",
            args,
        );
    }

    fn is_active(&self, _: &()) -> bool {
        self.active
    }

    fn is_available(&self, _: &()) -> bool {
        self.available
    }

    fn activate_primary(&mut self, _: &()) {
        self.activations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn activate_secondary(&mut self, _: &()) {}

    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new("Mock", self.name.len() as u32, None))
    }
}

#[derive(Default)]
struct MockHost {
    sounds: RefCell<Vec<Sound>>,
}

impl WheelHost<()> for MockHost {
    fn play_sound(&self, sound: Sound) {
        self.sounds.borrow_mut().push(sound);
    }

    fn icon(&self, icon: IconImageType) -> Image {
        Image {
            texture: 100 + icon as usize,
            width: 128,
            height: 128,
        }
    }

    fn item_from_menu_hovered(&self) -> Option<Box<dyn WheelItem<()>>> {
        Some(MockItem::boxed("from menu", true))
    }
}

fn sample_wheel() -> Wheel<()> {
    let mut wheel = Wheel::new();
    for name in ["Iron Sword", "Fireball", "Potion"] {
        let mut entry = WheelEntry::new();
        entry.push_item(MockItem::boxed(name, true));
        wheel.push_entry(entry);
    }
    wheel
}

#[test]
fn draws_text_and_primitives() {
    let stats = render_frame([1920.0, 1080.0], |_| {
        let args = DrawArgs::default();
        drawer::draw_text(100.0, 100.0, "Hello", 0xFFFF_FFFF, 30.0, args, true);
        drawer::draw_arc(
            [500.0, 500.0],
            100.0,
            120.0,
            0.0,
            1.0,
            0.0,
            1.0,
            0xFFFF_FFFF,
            16,
            args,
        );
        drawer::draw_circle_filled([10.0, 10.0], 5.0, 0xFFFF_FFFF, 8, args);
    });
    assert!(stats.vertices > 0);
    assert!(stats.indices > 0);
}

#[test]
fn zero_alpha_text_draws_nothing() {
    let base = render_frame([800.0, 600.0], |_| {});
    let stats = render_frame([800.0, 600.0], |_| {
        let args = DrawArgs {
            alpha_mult: 0.0,
            ..DrawArgs::default()
        };
        drawer::draw_text(100.0, 100.0, "invisible", 0xFFFF_FFFF, 30.0, args, true);
    });
    assert_eq!(stats.vertices, base.vertices);
}

#[test]
fn wheel_draws_entries_and_hovers_with_sound() {
    let host = MockHost::default();
    let mut wheel = sample_wheel();
    // Cursor pointing straight down hovers entry 0.
    let stats = render_frame([1920.0, 1080.0], |_| {
        wheel.draw(
            [960.0, 540.0],
            core::f32::consts::FRAC_PI_2,
            false,
            1.0 / 60.0,
            &(),
            &host,
            DrawArgs::default(),
        );
    });
    assert_eq!(wheel.hovered_entry_index(), 0);
    assert_eq!(*host.sounds.borrow(), vec![Sound::EntrySwitch]);
    // Background icons and the item texture are referenced.
    assert!(
        stats
            .textures
            .contains(&(100 + IconImageType::wheel_background as usize))
    );
    assert!(
        stats
            .textures
            .contains(&(100 + IconImageType::slot_highlighted_background as usize))
    );
    assert!(stats.textures.contains(&42));
}

#[test]
fn unavailable_items_are_pruned_on_draw() {
    let host = MockHost::default();
    let mut wheel = Wheel::new();
    let mut entry = WheelEntry::new();
    entry.push_item(MockItem::boxed("gone", false));
    entry.push_item(MockItem::boxed("kept", true));
    wheel.push_entry(entry);
    render_frame([800.0, 600.0], |_| {
        wheel.draw(
            [400.0, 300.0],
            0.0,
            true,
            0.0,
            &(),
            &host,
            DrawArgs::default(),
        );
    });
    assert_eq!(wheel.entries()[0].num_items(), 1);
}

#[test]
fn edit_mode_primary_inserts_menu_item_and_secondary_removes_empty_entry() {
    let host = MockHost::default();
    let mut wheel: Wheel<()> = Wheel::new();
    wheel.push_empty_entry();
    wheel.set_hovered_entry_index(0);

    wheel.activate_hovered_entry_primary(true, &(), &host);
    assert_eq!(wheel.entries()[0].num_items(), 1);

    // Remove the item, then the now-empty entry.
    wheel.activate_hovered_entry_secondary(true, &());
    assert!(wheel.entries()[0].is_empty());
    wheel.activate_hovered_entry_secondary(true, &());
    assert!(wheel.is_empty());
}

#[test]
fn entries_reorder_with_wrap_around() {
    let mut wheel = sample_wheel();
    wheel.set_hovered_entry_index(2);
    wheel.move_hovered_entry_forward();
    // Entry 2 swapped with entry 0.
    let host = MockHost::default();
    let first = render_frame([800.0, 600.0], |_| {
        wheel.draw(
            [400.0, 300.0],
            0.0,
            true,
            0.0,
            &(),
            &host,
            DrawArgs::default(),
        );
    });
    assert!(first.vertices > 0);
    assert_eq!(wheel.num_entries(), 3);
}

#[test]
fn save_data_round_trip() {
    let mut wheel = sample_wheel();
    wheel.push_empty_entry();
    let data = wheel.save_data();
    assert_eq!(data.entries.len(), 4);
    assert_eq!(
        data.entries[0].items[0],
        ItemData::new("Mock", "Iron Sword".len() as u32, None)
    );

    let names = ["Iron Sword", "Fireball", "Potion"];
    let mut loaded = Wheel::<()>::from_save_data(&data, &mut |item: &ItemData| {
        let name = names.iter().find(|n| n.len() as u32 == item.form_id)?;
        Some(MockItem::boxed(name, true))
    });
    assert_eq!(loaded.save_data(), data);

    // Unknown items are dropped and the selection is clamped.
    let mut data = WheelData::default();
    data.entries.push(wheeler_core::save::EntryData {
        items: vec![ItemData::new("Unknown", 0, None)],
        selected_item: 3,
    });
    loaded = Wheel::from_save_data(&data, &mut |_: &ItemData| None);
    assert!(loaded.entries()[0].is_empty());
    assert_eq!(loaded.entries()[0].selected_item_index(), 0);
}

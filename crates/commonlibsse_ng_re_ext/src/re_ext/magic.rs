use commonlibsse_ng::re::BSString::BSString;
use commonlibsse_ng::re::ItemCard::ItemCard;
use commonlibsse_ng::re::MagicItem::MagicItem;

#[commonlibsse_ng::relocate_fn(se_id = 51022, ae_id = 51900)]
#[inline]
pub fn get_magic_item_description(
    item_card: *mut ItemCard,
    magic_item: *mut MagicItem,
    string: &BSString,
) {
}

pub fn strip_magic_item_description_format_code(description: &mut String) {
    while let Some(bracket_lhs) = description.find('<') {
        if let Some(bracket_rhs) = description[bracket_lhs..].find('>') {
            let bracket_rhs = bracket_lhs + bracket_rhs;
            description.drain(bracket_lhs..=bracket_rhs);
        } else {
            break;
        }
    }
}

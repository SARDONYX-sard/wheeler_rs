use core::ptr::NonNull;

use commonlibsse_ng::re::BSCoreTypes::FormID;
use commonlibsse_ng::re::BSTArray::BSTArray;
use commonlibsse_ng::re::BSTList::BSSimpleList;
use commonlibsse_ng::re::FormTypes::FormType;
use commonlibsse_ng::re::TESFile::TESFile;
use commonlibsse_ng::re::TESForm::TESForm;
use commonlibsse_ng::re::TESObjectCELL::TESObjectCELL;
use commonlibsse_ng::rel::module::is_vr;

#[derive(Debug)]
#[repr(C)]
pub struct TESObjectList {
    pad0: u8, // 0
}
const _: () = assert!(core::mem::size_of::<TESObjectList>() == 0x1);

#[derive(Debug)]
#[repr(C)]
pub struct TESFileCollection {
    pub files: BSTArray<*mut TESFile>,      // 0x000
    pub smallFiles: BSTArray<*mut TESFile>, // 0x018
}
const _: () = assert!(core::mem::size_of::<TESFileCollection>() == 0x30);

#[derive(Debug)]
#[repr(C)]
pub struct BGSPrimitive;
#[derive(Debug)]
#[repr(C)]
pub struct InventoryChanges;
#[derive(Debug)]
#[repr(C)]
pub struct TESRegionList;
#[derive(Debug)]
#[repr(C)]
pub struct BGSAddonNode;

#[derive(Debug)]
#[repr(C)]
pub struct NiTPrimitiveArray<T> {
    opaque: [u8; 0x18],
    marker: core::marker::PhantomData<T>,
}
#[derive(Debug)]
#[repr(C)]
pub struct NiTList<T> {
    opaque: [u8; 0x18],
    marker: core::marker::PhantomData<T>,
}

#[derive(Debug)]
#[repr(C)]
pub struct TESDataHandler {
    // pub __base: BSTSingletonSDM<TESDataHandler> // size is 0
    pub pad001: u8,                                           // 0x001
    pub pad002: u16,                                          // 0x002
    pub pad004: u32,                                          // 0x004
    pub objectList: *mut TESObjectList,                       // 0x008
    pub formArrays: [BSTArray<*mut TESForm>; 138],            // 0x010 - 138: `FormType::Max`
    pub regionList: *mut TESRegionList,                       // 0xD00
    pub interiorCells: NiTPrimitiveArray<*mut TESObjectCELL>, // 0xD08
    pub addonNodes: NiTPrimitiveArray<*mut BGSAddonNode>,     // 0xD20
    pub badForms: NiTList<*mut TESForm>,                      // 0xD38
    pub nextID: FormID,                                       // 0xD50
    pub padD54: u32,                                          // 0xD54
    pub activeFile: *mut TESFile,                             // 0xD58
    pub files: BSSimpleList<*mut TESFile>,                    // 0xD60
}
const _: () = {
    assert!(FormType::Max as usize == 138);
    assert!(core::mem::size_of::<TESDataHandler>() == 0xD70);
};

impl TESDataHandler {
    /// Returns the singleton instance of `Self`.
    #[commonlibsse_ng::relocate(
        cast_as = "*mut TESDataHandler",
        default = "None",
        id(se = 514141, ae = 400269)
    )]
    pub fn get_singleton() -> Option<&'static TESDataHandler> {
        |as_type: AsType| unsafe { as_type.as_ref() }
    }

    /// Returns the singleton instance of `Self`.
    ///
    /// # Safety
    /// When it is known whether this is mutable or not. (The author does not know if it is safe.)
    #[commonlibsse_ng::relocate(
        cast_as = "*mut TESDataHandler",
        default = "None",
        id(se = 514141, ae = 400269)
    )]
    pub unsafe fn get_singleton_mut() -> Option<&'static mut TESDataHandler> {
        |as_type: AsType| unsafe { as_type.as_mut() }
    }

    pub fn lookup_form(&self, local_form_id: FormID, mod_name: &str) -> Option<NonNull<TESForm>> {
        let form_id = self.lookup_form_id(local_form_id, mod_name)?;
        TESForm::lookup_by_id(form_id)
    }

    pub fn lookup_form_id(&self, local_form_id: FormID, mod_name: &str) -> Option<FormID> {
        let file = self.lookup_by_name(mod_name)?;
        let file_compile_index = file.compileIndex as u32;
        if file_compile_index == 0xFF {
            return None;
        };

        let mut form_id = file_compile_index << 24;
        let local_form_id = local_form_id.get();

        let ret_form_id = if is_vr() {
            //  Use SkyrimVR lookup logic, ignore light plugin index which doesn't exist in VR
            (local_form_id & 0xFFFFFF) | form_id
        } else {
            form_id += (file.smallFileCompileIndex as u32) << (8 + 4);
            form_id + local_form_id
        };

        Some(FormID::new(ret_form_id))
    }

    pub fn lookup_by_name(&self, mod_name: &str) -> Option<&TESFile> {
        for file in &self.files {
            let file = unsafe { file.as_ref() };

            if file
                .map(|file| file.fileName.as_slice() == mod_name.as_bytes())
                .unwrap_or_default()
            {
                return file;
            }
        }

        None
    }
}

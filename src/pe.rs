//! Read-only PE parsing: enough to walk sections of a loaded module or of a
//! file on disk, and to address every executable region.
//!
//! Both an in-memory image (the running `nightreign.exe`) and a file mapped
//! from disk (used by the `deep-scan` verification tool) are supported
//! through the same `ExecRegion` abstraction.

#![allow(dead_code)]

use core::ffi::c_void;
use core::slice;

pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
pub const IMAGE_SCN_CNT_CODE: u32 = 0x0000_0020;
pub const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
pub const IMAGE_NT_SIGNATURE: u32 = 0x0000_4550;
pub const PE32_MAGIC: u16 = 0x010B;
pub const PE32_PLUS_MAGIC: u16 = 0x020B;

#[derive(Debug, Clone, Copy)]
pub struct Section {
    pub name: [u8; 8],
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_size: u32,
    pub raw_offset: u32,
    pub characteristics: u32,
}

impl Section {
    pub fn name_str(&self) -> String {
        let end = self.name.iter().position(|&c| c == 0).unwrap_or(8);
        String::from_utf8_lossy(&self.name[..end]).to_string()
    }

    pub fn is_executable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_EXECUTE != 0
    }

    /// Number of bytes that can safely be addressed in memory for this section.
    pub fn mapped_len(&self, image_size: u32) -> usize {
        let end = self
            .virtual_address
            .saturating_add(self.virtual_size.max(self.raw_size));
        let end = end.min(image_size.max(self.virtual_address));
        end.saturating_sub(self.virtual_address) as usize
    }

    /// Number of bytes that exist in the file image for this section.
    pub fn file_len(&self) -> usize {
        self.virtual_size.min(self.raw_size) as usize
    }
}

#[derive(Debug, Clone)]
pub struct Headers {
    pub image_base: u64,
    pub image_size: u32,
    pub section_alignment: u32,
    pub sections: Vec<Section>,
}

impl Headers {
    /// Parse the PE headers at the start of `data`, where `data[0]` corresponds
    /// to `image_base` in the address space.
    pub fn parse(data: &[u8], image_base: u64) -> Result<Headers, String> {
        let rd_u16 = |o: usize| -> Result<u16, String> {
            data.get(o..o + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or_else(|| format!("truncated at 0x{o:X}"))
        };
        let rd_u32 = |o: usize| -> Result<u32, String> {
            data.get(o..o + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| format!("truncated at 0x{o:X}"))
        };
        let rd_u64 = |o: usize| -> Result<u64, String> {
            data.get(o..o + 8)
                .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
                .ok_or_else(|| format!("truncated at 0x{o:X}"))
        };

        if rd_u16(0)? != IMAGE_DOS_SIGNATURE {
            return Err("not a PE image (bad MZ signature)".into());
        }
        let e_lfanew = rd_u32(0x3C)? as usize;
        if rd_u32(e_lfanew)? != IMAGE_NT_SIGNATURE {
            return Err("not a PE image (bad PE signature)".into());
        }

        let coff = e_lfanew + 4;
        let machine = rd_u16(coff)?;
        if machine != 0x8664 {
            return Err(format!(
                "unexpected machine 0x{machine:X} (expected x86-64)"
            ));
        }
        let number_of_sections = rd_u16(coff + 2)? as usize;
        let size_of_optional_header = rd_u16(coff + 16)? as usize;
        let opt = coff + 20;

        let magic = rd_u16(opt)?;
        let (image_size, section_alignment, base) = match magic {
            PE32_PLUS_MAGIC => (rd_u32(opt + 56)?, rd_u32(opt + 32)?, rd_u64(opt + 24)?),
            PE32_MAGIC => (
                rd_u32(opt + 56)?,
                rd_u32(opt + 32)?,
                rd_u32(opt + 28)? as u64,
            ),
            m => return Err(format!("unexpected optional header magic 0x{m:X}")),
        };

        let section_table = opt + size_of_optional_header;
        let mut sections = Vec::with_capacity(number_of_sections);
        for i in 0..number_of_sections {
            let s = section_table + i * 40;
            let mut name = [0u8; 8];
            name.copy_from_slice(data.get(s..s + 8).ok_or("truncated section table")?);
            sections.push(Section {
                name,
                virtual_size: rd_u32(s + 8)?,
                virtual_address: rd_u32(s + 12)?,
                raw_size: rd_u32(s + 16)?,
                raw_offset: rd_u32(s + 20)?,
                characteristics: rd_u32(s + 36)?,
            });
        }

        Ok(Headers {
            image_base: if base == 0 { image_base } else { base },
            image_size,
            section_alignment,
            sections,
        })
    }
}

/// One contiguous stretch of executable bytes, addressed by RVA.
pub struct ExecRegion<'a> {
    pub name: String,
    pub rva: u32,
    pub bytes: &'a [u8],
}

/// A loaded module (in-memory image).
pub struct ModuleImage {
    pub base: *const u8,
    pub headers: Headers,
}

impl ModuleImage {
    /// SAFETY: `base` must point at a valid, mapped PE image header.
    pub unsafe fn from_base(base: *const u8) -> Result<ModuleImage, String> {
        // The DOS header + NT headers + section table are always inside the
        // first page of the image.
        let head = slice::from_raw_parts(base, 0x1000);
        let headers = Headers::parse(head, base as u64)?;
        Ok(ModuleImage { base, headers })
    }

    pub unsafe fn exec_regions(&self) -> Vec<ExecRegion<'_>> {
        let mut out = Vec::new();
        for s in &self.headers.sections {
            if !s.is_executable() {
                continue;
            }
            let len = s.mapped_len(self.headers.image_size);
            if len == 0 {
                continue;
            }
            let p = self.base.add(s.virtual_address as usize);
            out.push(ExecRegion {
                name: s.name_str(),
                rva: s.virtual_address,
                bytes: slice::from_raw_parts(p, len),
            });
        }
        out
    }

    /// Convert an RVA into a live pointer inside the module.
    pub unsafe fn rva_ptr(&self, rva: u32) -> *const u8 {
        self.base.add(rva as usize)
    }
}

/// A PE file mapped into a `Vec<u8>` (no in-memory image semantics).
pub struct FileImage<'a> {
    pub data: &'a [u8],
    pub headers: Headers,
}

impl<'a> FileImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<FileImage<'a>, String> {
        let headers = Headers::parse(data, 0)?;
        Ok(FileImage { data, headers })
    }

    pub fn exec_regions(&self) -> Vec<ExecRegion<'a>> {
        let mut out = Vec::new();
        for s in &self.headers.sections {
            if !s.is_executable() {
                continue;
            }
            let off = s.raw_offset as usize;
            let len = s.file_len();
            if len == 0 || off >= self.data.len() {
                continue;
            }
            let end = (off + len).min(self.data.len());
            out.push(ExecRegion {
                name: s.name_str(),
                rva: s.virtual_address,
                bytes: &self.data[off..end],
            });
        }
        out
    }
}

/// `GetModuleHandleW(NULL)` — the process' main module (the game executable).
pub fn main_module_base() -> Result<*const u8, String> {
    let h = unsafe { crate::win::GetModuleHandleW(core::ptr::null()) };
    if h.is_null() {
        return Err("GetModuleHandleW(NULL) failed".into());
    }
    Ok(h as *const u8)
}

/// Full path of a loaded module, resolved through `GetModuleFileNameW`.
pub fn module_path(hmodule: *mut c_void) -> Result<std::path::PathBuf, String> {
    let mut buf = vec![0u16; 32_768];
    let n = unsafe { crate::win::GetModuleFileNameW(hmodule, buf.as_mut_ptr(), buf.len() as u32) };
    if n == 0 {
        return Err("GetModuleFileNameW failed".into());
    }
    buf.truncate(n as usize);
    Ok(std::path::PathBuf::from(String::from_utf16_lossy(&buf)))
}

/// Resolve the base address of the module that contains `addr`.
pub fn module_base_from_address(addr: *const c_void) -> Result<*const u8, String> {
    let mut h: crate::win::HMODULE = core::ptr::null_mut();
    let ok = unsafe {
        crate::win::GetModuleHandleExW(
            crate::win::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                | crate::win::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            addr as *const u16,
            &mut h,
        )
    };
    if ok == 0 || h.is_null() {
        return Err("GetModuleHandleExW failed".into());
    }
    Ok(h as *const u8)
}

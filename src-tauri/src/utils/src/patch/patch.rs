use crate::errors::Result;
use crate::patch::errors::UPatchError;
use crate::patch::types::Bytes;
use crate::patch::types::Hex;
use crate::patch::types::PatchDataType;
use crate::patch::types::PatchType;
use aobscan::PatternBuilder;
use log::info;
use log::warn;
use memmap2::Mmap;
use memmap2::MmapMut;
use pelite::PeFile;
use core::option::Option::Some;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

pub struct UPatch {
    data: PatchDataType,
    file: String,
    save: String,
    with_write: bool,
    sections: Option<Vec<(u64, u64, u64)>>,
}

impl UPatch {
    pub fn create(input: &str, save: &str, with_write: bool) -> Result<Self> {
        let data = match with_write {
            true => {
                // 另存为时应使用 fs::read，mmap_mut 在 drop 时 flush 源文件？
                if save == input {
                    Self::open_with_map_mut(input)
                } else {
                    Self::open_with_fs(input)
                }
            }
            false => Self::open_with_map(input),
        };
        let data = match data {
            Ok(data) => data,
            Err(e) => {
                warn!("{}", e);
                Self::open_with_fs(input)?
            }
        };
        return Self::new(data, input, save, with_write);
    }

    pub fn new(data: PatchDataType, file: &str, save: &str, with_write: bool) -> Result<Self> {
        let sections = if let Ok(secs) = Self::init_sections(&data){
            Some(secs)
        }else{
            None
        };
        Ok(Self {
            data,
            file: file.to_string(),
            save: save.to_string(),
            with_write,
            sections: sections,
        })
    }

    fn open_with_map(input: &str) -> Result<PatchDataType> {
        let file = OpenOptions::new().read(true).open(input)?;
        let data = unsafe { Mmap::map(&file) }
            .map_err(|_| UPatchError::ReadWithMmapError(input.to_string()))?;
        Ok(PatchDataType::Mmap(data))
    }

    fn open_with_map_mut(input: &str) -> Result<PatchDataType> {
        let file = OpenOptions::new().read(true).write(true).open(input)?;
        let data = unsafe { MmapMut::map_mut(&file) }
            .map_err(|_| UPatchError::WriteWithMmapMutError(input.to_string()))?;
        Ok(PatchDataType::MmapMut(data))
    }

    fn open_with_fs(input: &str) -> Result<PatchDataType> {
        let data = std::fs::read(input)?;
        Ok(PatchDataType::Data(data))
    }

    pub fn get_data(&self) -> &[u8] {
        Self::get_data_by_datetype(&self.data)
    }

    pub fn get_data_by_datetype(data: &PatchDataType) -> &[u8] {
        match &data {
            PatchDataType::Mmap(data) => data,
            PatchDataType::MmapMut(data) => data,
            PatchDataType::Data(data) => data,
        }
    }

    pub fn get_file(&self) -> &str {
        self.file.as_str()
    }

    pub fn get_save(&self) -> &str {
        self.save.as_str()
    }

    pub fn check_pos(&self, pos: usize, len: usize) -> Result<(usize, usize)> {
        if pos > self.get_data().len() {
            return Err(UPatchError::OutRangePos1Error.into());
        }
        let pos2 = pos + len;
        if pos + len > self.get_data().len() {
            return Err(UPatchError::OutRangePos2Error.into());
        }
        Ok((pos, pos2))
    }

    pub fn len(&self) -> usize {
        self.get_data().len()
    }

    pub fn read(&self, pos: usize, len: usize) -> Result<Bytes> {
        let (pos1, pos2) = self.check_pos(pos, len)?;
        Ok(Bytes::new(self.get_data()[pos1..pos2].to_vec()))
    }

    pub fn read_hex(&self, pos: usize, len: usize) -> Result<String> {
        Ok(self.read(pos, len)?.to_hex())
    }

    pub fn read_utf8(&self, pos: usize, len: usize) -> Result<String> {
        self.read(pos, len)?.to_utf8()
    }

    pub fn search(&self, pattern: &str) -> Result<Vec<usize>> {
        self.search_by_pattern(pattern, false)
    }

    pub fn search_all(&self, pattern: &str) -> Result<Vec<usize>> {
        self.search_by_pattern(pattern, true)
    }

    fn search_by_pattern(&self, pattern: &str, all: bool) -> Result<Vec<usize>> {
        if pattern.is_empty() || pattern.len() % 2 != 0 {
            return Err(UPatchError::PatternBuilderError.into());
        }
        let pattern = PatternBuilder::from_hex_string(pattern)
            .map_err(|_| UPatchError::PatternBuilderError)?
            .with_all_threads()
            .build();

        let mut results = Vec::new();
        let data = self.get_data();
        pattern.scan(data, |offset| {
            if offset < data.len() {
                results.push(offset);
            }
            all
        });
        if results.is_empty() {
            return Err(UPatchError::PatternNotFindError.into());
        }
        Ok(results)
    }

    pub fn init_sections(data: &PatchDataType) -> Result<Vec<(u64, u64, u64)>> {

        let pe_data = Self::get_data_by_datetype(data);
        let pe_file = PeFile::from_bytes(pe_data).map_err(|_| UPatchError::FOAToRVAError)?;
        let sections_headers = pe_file.section_headers();
        let mut sections = Vec::new();
        for section in sections_headers {
            let range = section.file_range();
            sections.push((
                range.start as u64,
                range.end as u64,
                section.VirtualAddress as u64,
            ));
        }
        Ok(sections)
    }

    pub fn foa_to_rva(&self, foa: u64) -> Result<u64> {
        let sections = &self.sections;
        if let Some(sections) = sections {
            for section in sections {
                let section_start = section.0;
                let section_end = section.1;
                let v_address = section.2;
                // 处理未映射到节的数据（如PE头）
                if foa < section_start {
                    return Ok(foa);
                }
                // 检查 FOA 是否在当前节内
                if foa >= section_start && foa < section_end {
                    // 计算节内偏移
                    let offset_in_section = foa - section_start;
                    // 转换为 RVA: 节起始RVA + 节内偏移
                    return Ok(v_address + offset_in_section);
                }
            }
        }
        Err(UPatchError::FOAToRVAError.into())
    }

    pub fn write(&mut self, pos: usize, data: PatchType) -> Result<&Self> {
        if !self.with_write {
            return Err(UPatchError::ReadOnlyError.into());
        }
        let new_data = match data {
            PatchType::String(string) => Hex::new(string).try_to_bytes()?,
            PatchType::Data(data) => Bytes::new(data),
        };
        let len = new_data.len();
        let (pos1, pos2) = self.check_pos(pos, len)?;
        let new_data_bytes = new_data.as_bytes();

        match &mut self.data {
            PatchDataType::Mmap(_) => {
                return Err(UPatchError::ReadOnlyError.into());
            }
            PatchDataType::MmapMut(mmap_mut) => {
                mmap_mut[pos1..pos2].copy_from_slice(new_data_bytes);
            }
            PatchDataType::Data(data) => {
                data[pos1..pos2].copy_from_slice(new_data_bytes);
            }
        }
        Ok(self)
    }

    pub fn save(&self) -> Result<()> {
        if !self.with_write {
            // return Err(UPatchError::ReadOnlyError.into());
            // 跳过保存
            return Ok(());
        }

        match self.file.as_str() == self.save.as_str() {
            true => self.save_to(),
            false => self.save_as(),
        }
    }

    fn save_to(&self) -> Result<()> {
        info!("正在保存文件：{}", self.save);
        match &self.data {
            PatchDataType::Mmap(_) => return Err(UPatchError::ReadOnlyError.into()),
            PatchDataType::Data(data) => std::fs::write(&self.save, data)?,
            PatchDataType::MmapMut(mmap_mut) => mmap_mut.flush()?,
        }
        Ok(())
    }

    fn save_as(&self) -> Result<()> {
        info!("正在另存为文件：{}", self.save);
        let mut new_file = File::create(self.save.as_str())?;
        new_file.write_all(self.get_data())?;
        new_file.sync_all()?;
        Ok(())
    }
}

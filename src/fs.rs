use core::{ops::{Index, Range, IndexMut}, borrow::Borrow};

use alloc::{collections::{BTreeMap}, string::{String, FromUtf8Error}, vec::{Vec}, slice};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum FsError {
    FileNotFound
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
enum DirectoryType {
    File,
    Folder,
    Url
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct Directory {
    variant: DirectoryType,
    name: String
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(Vec<Directory>);

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Permissions {
    read: bool,
    write: bool,
    execute: bool
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct File {
    permissions: Permissions,
    contents: Vec<u8>
}

impl File {
    pub fn overwrite(&mut self, new_content: Vec<u8>) {
        self.contents = new_content
    }

    pub fn read_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.contents.clone())
    }
}

impl Index<usize> for File {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}

impl Index<Range<usize>> for File {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.contents[index]
    }
}

/// A trait that filesystem drivers can implement to support all base SprinklesOS read/write operations
pub trait Filesystem:
    Index<Path, Output = File> + 
    IndexMut<Path>
{
    fn init() -> Self;
    
    fn read_file(file: impl Borrow<File>) -> Result<String, FromUtf8Error> {
        String::from_utf8(file.borrow().contents.clone())
    }

    fn get_dir(&self, path: Path) -> Option<&File> {
        self.index(path).into()
    }

    fn read_dir(&self, path: Path) -> Option<slice::Iter<u8>> {
        let Some(file) = self.get_dir(path) else {
            return None;
        };

        Some(file.contents.iter())
    }

    fn write_dir(&mut self, path: Path, content: Vec<u8>) -> Result<(), FsError> {
        let file_ref = self.index_mut(path);

        file_ref.contents = content;

        Ok(())
    }
}

/// A dummy filesystem that exclusively writes to the memory.
pub struct MemoryFS {
    /// The key is the filename (Path), the value is the file
    items: BTreeMap<Path, File>
}

impl Index<Path> for MemoryFS {
    type Output = File;

    fn index(&self, path: Path) -> &Self::Output {
        self.items.get(&path).unwrap()
    }
}

impl IndexMut<Path> for MemoryFS {
    fn index_mut(&mut self, path: Path) -> &mut Self::Output {
        self.items.get_mut(&path).unwrap()
    }
}

impl Filesystem for MemoryFS {
    fn init() -> MemoryFS {
        MemoryFS {
            items: BTreeMap::default()
        }
    }
}
use errors::OpaqueError;
use std::io::{ BufReader, BufWriter, Read, Write };
use std::path::Path;
use std::fs::{ self, File };

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String, OpaqueError> {
    let path = path.as_ref();
    let mut br = BufReader::new(File::open(path)?);
    let mut result = String::new();
    br.read_to_string(&mut result)?;
    Ok(result)
}

pub fn read_file_raw<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, OpaqueError> {
    let path = path.as_ref();
    let mut br = BufReader::new(File::open(path)?);
    let mut result = Vec::new();
    br.read_to_end(&mut result)?;
    Ok(result)
}

pub fn write_file<P, B>(path: P, content: B) -> Result<(), OpaqueError>
    where P: AsRef<Path>,
          B: AsRef<[u8]>
{
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut br = BufWriter::new(File::create(path)?);
    br.write_all(content.as_ref())?;
    Ok(())
}

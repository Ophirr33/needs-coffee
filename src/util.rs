use errors::OResult;
use std::io::{ BufReader, BufWriter, Read, Write };
use std::path::Path;
use std::fs::{ self, File };
use html5ever_ext::{RcDom, RcDomExt, Minify};

pub fn read_file<P: AsRef<Path>>(path: P) -> OResult<String> {
    let path = path.as_ref();
    let mut br = BufReader::new(File::open(path)?);
    let mut result = String::new();
    br.read_to_string(&mut result)?;
    Ok(result)
}

pub fn write_minified_html<P, B>(path: P, content: B) -> OResult<()>
    where P: AsRef<Path>,
          B: AsRef<[u8]>
{
    let dom = RcDom::from_bytes(content.as_ref());
    dom.minify_to_file_path(false, path)?;
    Ok(())
}

pub fn write_file<P, B>(path: P, content: B) -> OResult<()>
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

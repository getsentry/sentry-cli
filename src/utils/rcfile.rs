use std::fs;
use std::io::{Seek, SeekFrom, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use java_properties::{PropertiesIter, PropertiesWriter};
use ini::Ini;
use encoding::Encoding;
use encoding::all::{UTF_8, ISO_8859_1};

use prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RcFileFormat {
    Ini,
    Properties,
}


pub struct RcFile {
    filename: Option<PathBuf>,
    format: RcFileFormat,
    items: HashMap<String, String>,
}

fn load_props<R: Read + Seek>(mut rdr: R, encoding: &'static Encoding)
    -> Result<HashMap<String, String>>
{
    let mut rv = HashMap::new();
    rdr.seek(SeekFrom::Start(0))?;
    PropertiesIter::new_with_encoding(rdr, encoding).read_into(|key, value| {
        rv.insert(key, value);
    }).map_err(|_| Error::from("bad property data"))?;
    Ok(rv)
}

impl RcFile {
    pub fn new() -> RcFile {
        RcFile {
            filename: None,
            format: RcFileFormat::Ini,
            items: HashMap::new(),
        }
    }

    pub fn open<R: Read + Seek>(mut rdr: R) -> Result<RcFile> {
        let format;
        let mut items;

        // try to load as utf-8 props first
        if let Ok(rv) = load_props(&mut rdr, UTF_8) {
            format = RcFileFormat::Properties;
            items = rv;
        } else if let Ok(rv) = load_props(&mut rdr, ISO_8859_1) {
            format = RcFileFormat::Properties;
            items = rv;
        } else {
            format = RcFileFormat::Ini;
            items = HashMap::new();

            let ini = Ini::read_from(&mut rdr)?;
            for (section, props) in ini.iter() {
                for (key, value) in props {
                    items.insert(match *section {
                        Some(ref section) => format!("{}.{}", section, key),
                        None => key.to_owned()
                    }, value.to_owned());
                }
            }
        }

        Ok(RcFile {
            filename: None,
            format: format,
            items: items,
        })
    }

    pub fn filename(&self) -> Option<&Path> {
        self.filename.as_ref().map(|x| x.as_path())
    }

    pub fn set_filename<P: AsRef<Path>>(&mut self, path: Option<P>) {
        self.filename = path.map(|x| x.as_ref().to_path_buf());
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<RcFile> {
        let f = BufReader::new(fs::File::open(path.as_ref())?);
        let mut rv = RcFile::open(f)?;
        rv.filename = Some(path.as_ref().to_path_buf());
        Ok(rv)
    }

    pub fn save(&self) -> Result<()> {
        let filename = self.filename.as_ref().expect("No filename set");
        let file = fs::OpenOptions::new().write(true)
            .truncate(true)
            .create(true)
            .open(filename)?;
        self.to_writer(file)
    }

    pub fn to_writer<W: Write>(&self, mut writer: W) -> Result<()> {
        let mut items: Vec<(&str, &str)> = self.items
            .iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        items.sort();

        match self.format {
            RcFileFormat::Ini => {
                let mut ini = Ini::new();
                for &(key, value) in &items {
                    let mut key_iter = key.splitn(2, '.');
                    let (sect, key) = match (key_iter.next(), key_iter.next()) {
                        (Some(sect), Some(key)) => (Some(sect), key),
                        (Some(key), None) => (None, key),
                        _ => continue
                    };
                    ini.set_to(sect, key.to_string(), value.to_string());
                }
                ini.write_to(&mut writer)?;
                Ok(())
            }
            RcFileFormat::Properties => {
                let mut w = PropertiesWriter::new(&mut writer);
                for &(key, value) in &items {
                    w.write(&key, &value)
                        .map_err(|_| Error::from("could not write properties"))?;
                }
                Ok(())
            }
        }
    }

    pub fn format(&self) -> RcFileFormat {
        self.format
    }

    pub fn set_format(&mut self, format: RcFileFormat) {
        self.format = format;
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(|x| x.as_str())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }
}

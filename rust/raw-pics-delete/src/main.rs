#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate failure;
extern crate clap;

use clap::{App, Arg, SubCommand};
use failure::Error;

use std::collections::HashSet;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

fn main() {
    env_logger::init();
    let matches = App::new("Delete raws of photos")
                          .version("1.0")
                          .author("Jonathan Fok kan <jfokkan@gmail.com>")
                          .about("Deletes the .RAF files without corresponding JPG file")
                          .arg(Arg::with_name("DIR")
                               .help("Sets the input directory to use")
                               .required(true)
                               .index(1))
                          .arg(Arg::with_name("delete")
                               .short("d")
                               .long("delete")
                               .help("Sets whether to delete the raw files. Otherwise the default behaviour is to print the files without deleting"))
                          .get_matches();

    let dir = matches.value_of("DIR").unwrap();
    debug!("Deleting raws from {}", dir);
    let delete = matches.occurrences_of("delete") > 0;
    debug!("Delete enabled {}", delete);

    let extra_raws = read_dir_for_extra_raws(dir);

    if delete {
        for raw in extra_raws {
            debug!("Deleting {}", raw.display());
            fs::remove_file(raw).expect("failed to delete file");
        }
    } else {
        for raw in extra_raws {
            println!("{}", raw.display());
        }
    }
}

fn read_dir_for_extra_raws<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut jpgs = HashSet::new();
    let mut raws = HashSet::new();
    let entries_iter =
        fs::read_dir(&path).expect(&format!("failed reading path: {}", path.as_ref().display()));

    for entry in entries_iter {
        let entry = entry.expect("failed entry");

        let path = entry.path();
        if path.is_dir() {
            if is_jpg_dir(&path) {
                add_jpg_files(&path, &mut jpgs);
            } else if is_raw_dir(&path) {
                add_raw_files(&path, &mut raws);
            }
        }
    }

    add_jpg_files(path.as_ref(), &mut jpgs);
    add_raw_files(path.as_ref(), &mut raws);

    find_extra_raw_files(&jpgs, &raws)
}

fn find_extra_raw_files(jpgs: &HashSet<PathBuf>, raws: &HashSet<PathBuf>) -> Vec<PathBuf> {
    //let jpgs_stripped: HashSet<PathBuf> = jpgs.iter().map(|path| strip_extension(path)).collect();
    let mut extra_raws = Vec::new();

    for raw in raws.into_iter() {
        if !does_raw_have_corresponding_jpg(raw, &jpgs) {
            extra_raws.push(raw.clone());
        }
    }

    extra_raws
}

fn does_raw_have_corresponding_jpg(raw: &PathBuf, jpgs: &HashSet<PathBuf>) -> bool {
    jpgs.iter().any(|jpg| jpg.file_stem() == raw.file_stem())
}

fn strip_extension(path: &PathBuf) -> PathBuf {
    let extension = path
        .extension()
        .expect("failed to get extension from path when stripping extension")
        .to_str()
        .expect("failed to str");
    let str_path = path
        .to_str()
        .expect("failed path to str")
        .replace(&format!(".{}", extension), "");
    Path::new(&str_path).to_path_buf()
}

fn add_jpg_files(dir_path: &Path, jpgs: &mut HashSet<PathBuf>) {
    debug!("adding jpg files from {}", dir_path.display());
    let iter = fs::read_dir(&dir_path).expect("failed reading dir");

    for entry in iter {
        let entry = entry.expect("failed entry");
        let path = entry.path();
        if !path.is_dir() && is_jpg_file(&path) {
            jpgs.insert(path);
        }
    }
}

fn add_raw_files(dir_path: &Path, raws: &mut HashSet<PathBuf>) {
    debug!("adding raw files from {}", dir_path.display());
    let iter = fs::read_dir(&dir_path).expect("failed reading dir");

    for entry in iter {
        let entry = entry.expect("failed entry");
        let path = entry.path();
        if !path.is_dir() && is_raw_file(&path) {
            raws.insert(path);
        }
    }
}

fn is_jpg_dir<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().ends_with("jpg")
}

fn is_raw_dir<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().ends_with("raw")
}

fn is_jpg_file(path: &Path) -> bool {
    path.extension()
        .and_then(|os_ext| os_ext.to_str())
        .map(|ext| {
            let lower_ext = ext.to_lowercase();
            lower_ext == "jpg" || lower_ext == "jpeg"
        })
        .unwrap_or(false)
}

fn is_raw_file(path: &Path) -> bool {
    path.extension()
        .and_then(|os_ext| os_ext.to_str())
        .map(|ext| {
            let lower_ext = ext.to_lowercase();
            lower_ext == "raf"
        })
        .unwrap_or(false)
}

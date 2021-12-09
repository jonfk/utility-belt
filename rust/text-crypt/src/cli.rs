use std::path::Path;

use clap::{App, Arg, SubCommand};
use walkdir::{DirEntry, WalkDir};

mod check;
mod decrypt;
mod encrypt;

pub fn run() {
    // TODO: Upgrade to clap 3 to get bash completion generation
    let app = App::new("text-crypt")
        .version("1.0")
        .author("Jonathan Fok kan <jfokkan@gmail.com>")
        .about("Simple text encrypting program")
        .arg(
            Arg::with_name("v")
                .short("v")
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("encrypt")
                .aliases(&["e", "enc"])
                .about("Encrypt files containing \"---BEGIN CRYPT---\"")
                .arg(
                    Arg::with_name("password")
                        .env("PASS")
                        .short("p")
                        .required(true)
                        .help("password to be used"),
                )
                .arg(
                    Arg::with_name("INPUT")
                        .help("Path to the file to encrypt")
                        .min_values(0)
                        ,
                )
                .arg(
                    Arg::with_name("write")
                        .short("w")
                        .help("Write the result to the input file")
                        .takes_value(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("decrypt")
                .aliases(&["d", "dec"])
                .about("Decrypt files containing \"---BEGIN CRYPT---\"")
                .arg(
                    Arg::with_name("password")
                        .env("PASS")
                        .short("p")
                        .required(true)
                        .help("password to be used"),
                )
                .arg(
                    Arg::with_name("write")
                        .short("w")
                        .help("Write the result to the input file")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("files")
                        .help("Path to the files or directory to encrypt").min_values(0),
                ),
        )
        .subcommand(
            SubCommand::with_name("check")
                .aliases(&["c"])
                .about("Check that no files containing \"---BEGIN CRYPT---\" are unencrypted")
                .arg(Arg::with_name("files").help("Path to the files or directory to encrypt. Defaults to current directory if none is supplied").min_values(0)),
        );
    let matches = app.clone().get_matches();

    let verbose = matches.occurrences_of("v") > 0;

    if let Some(enc_matches) = matches.subcommand_matches("encrypt") {
        let password = enc_matches
            .value_of("password")
            .expect("password is required");
        let paths: Vec<_> = enc_matches.values_of("INPUT").unwrap_or_default().collect();
        let write_file = enc_matches.is_present("write");

        encrypt::encrypt_cmd(verbose, password, write_file, paths).expect("encrypt");
    } else if let Some(dec_matches) = matches.subcommand_matches("decrypt") {
        let password = dec_matches
            .value_of("password")
            .expect("password is required");
        let paths: Vec<_> = dec_matches.values_of("files").unwrap_or_default().collect();
        let write_file = dec_matches.is_present("write");

        decrypt::decrypt_cmd(verbose, write_file, password, paths).expect("decrypt");
    } else if let Some(check_matches) = matches.subcommand_matches("check") {
        let files: Vec<_> = check_matches
            .values_of("files")
            .unwrap_or_default()
            .collect();
        check::check_cmd(files).expect("check_files");
    } else {
        app.clone().print_help().expect("print help");
        std::process::exit(1);
    }
}
fn walk_dir<P: AsRef<Path>>(
    path: P,
) -> walkdir::FilterEntry<walkdir::IntoIter, fn(&DirEntry) -> bool> {
    WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| !is_hidden_or_binary(e))
}

fn is_hidden_or_binary(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| (!s.eq(".") && !s.eq("..") && s.starts_with(".")) || s.ends_with(".gpg"))
        .unwrap_or(false)
}

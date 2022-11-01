use clap::{Parser, Subcommand};
use error::CmdqError;
use std::{
    ffi::OsStr,
    fs::{self, File},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{event, info, span, Level};

pub mod error;
pub mod ytdlp;

fn main() -> Result<(), CmdqError> {
    tracing_subscriber::fmt::init();

    let cli_args = CliArgs::parse();

    match cli_args.commands {
        CliSubCommands::Ytdlp { filepath } => {
            let csv_file = File::open(&filepath).map_err(|err| CmdqError::FileOpenError {
                source: err,
                filepath: filepath.clone(),
            })?;
            let mut rdr = csv::Reader::from_reader(csv_file);

            let mut errored_records = Vec::new();

            for result in rdr.deserialize() {
                let record: ytdlp::Record =
                    result.map_err(|err| CmdqError::CsvDeserializeError { source: err })?;

                let span = span!(
                    Level::INFO,
                    "yt-dlp execute",
                    url = record.url,
                    title = record.title
                );
                let _enter = span.enter();

                event!(Level::INFO, "executing");
                match ytdlp::execute(&record.url, &record.title) {
                    Ok(_) => {
                        event!(Level::INFO, "execution succeeded");
                    }
                    Err(err) => {
                        event!(Level::ERROR, message = "execution failed", ?err);
                        errored_records.push(ErroredRecord { record, err });
                    }
                }
            }

            // TODO re-run errored records

            if errored_records.len() > 0 {
                write_errors(errored_records, &filepath)?;
            }

            fs::remove_file(&filepath).map_err(|err| CmdqError::RemoveInputFileError {
                source: err,
                filepath: filepath.clone(),
            })?;
        }
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(name = "cmdq")]
#[command(about = "A program to queue commands", long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    commands: CliSubCommands,
}

#[derive(Debug, Subcommand)]
enum CliSubCommands {
    Ytdlp { filepath: String },
}

struct ErroredRecord {
    record: ytdlp::Record,
    err: CmdqError,
}

fn error_filepath(filepath: &str) -> PathBuf {
    let mut path = PathBuf::from(filepath);
    let filename_without_ext = path
        .file_name()
        .expect("error_filepath error: filepath does not end in file_name")
        .to_str()
        .unwrap()
        .trim_end_matches(path.extension().unwrap_or(OsStr::new("")).to_str().unwrap());

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX EPOCH!")
        .as_secs();
    let error_filename = format!("{}-{}-err.csv", filename_without_ext, timestamp);

    path.set_file_name(error_filename);
    path
}

fn write_errors(errors: Vec<ErroredRecord>, filepath: &str) -> Result<(), CmdqError> {
    let error_filepath = error_filepath(&filepath);
    info!("writing errors to {}", error_filepath.display());

    let error_file =
        File::create(&error_filepath).map_err(|err| CmdqError::CreateErrorFileError {
            source: err,
            filepath: error_filepath.clone(),
        })?;

    let mut wtr = csv::Writer::from_writer(error_file);
    wtr.write_record(&["url", "title", "error"])
        .map_err(|err| CmdqError::WriteToErrorFileError {
            source: err,
            filepath: error_filepath.clone(),
        })?;
    for errored_record in errors {
        wtr.write_record(&[
            errored_record.record.url,
            errored_record.record.title,
            errored_record.err.to_string(),
        ])
        .map_err(|err| CmdqError::WriteToErrorFileError {
            source: err,
            filepath: error_filepath.clone(),
        })?;
    }
    wtr.flush().map_err(|err| CmdqError::WriteErrorFileError {
        source: err,
        filepath: error_filepath.clone(),
    })?;
    Ok(())
}

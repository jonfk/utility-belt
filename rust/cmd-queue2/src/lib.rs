use error::CmdqError;
use std::{
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{event, span, Level};

pub mod error;
pub mod ytdlp;

pub fn run_ytdlp_file(filepath: PathBuf) -> Result<(), CmdqError> {
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
        match ytdlp::execute(&filepath, &record) {
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
    Ok(())
}

struct ErroredRecord {
    record: ytdlp::Record,
    err: CmdqError,
}

fn error_filepath<T: AsRef<Path>>(filepath: T) -> PathBuf {
    let mut path = filepath.as_ref().to_path_buf();
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

fn write_errors<T: AsRef<Path>>(errors: Vec<ErroredRecord>, filepath: T) -> Result<(), CmdqError> {
    let error_filepath = error_filepath(&filepath);
    event!(
        Level::WARN,
        message = "writing errors",
        path = format!("{}", error_filepath.display())
    );

    let error_file =
        File::create(&error_filepath).map_err(|err| CmdqError::CreateErrorFileError {
            source: err,
            filepath: error_filepath.clone(),
        })?;

    let mut wtr = csv::Writer::from_writer(error_file);
    wtr.write_record(&["url", "title", "dir", "error"])
        .map_err(|err| CmdqError::WriteToErrorFileError {
            source: err,
            filepath: error_filepath.clone(),
        })?;
    for errored_record in errors {
        wtr.serialize((
            errored_record.record.url,
            errored_record.record.title,
            errored_record.record.dir,
            errored_record.err.to_string(),
        ))
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

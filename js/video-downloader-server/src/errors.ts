export abstract class AppError extends Error {
  abstract readonly code: string;
  abstract readonly statusCode: number;
  readonly details?: Record<string, unknown>;

  constructor(message: string, details?: Record<string, unknown>) {
    super(message);
    this.name = this.constructor.name;
    this.details = details;
    Error.captureStackTrace(this, this.constructor);
  }
}

export class InvalidUrlError extends AppError {
  readonly code = 'INVALID_URL';
  readonly statusCode = 400;
}

export class UnsupportedUrlError extends AppError {
  readonly code = 'UNSUPPORTED_URL';
  readonly statusCode = 400;
}

export class NameResolutionError extends AppError {
  readonly code = 'NAME_RESOLUTION_FAILED';
  readonly statusCode = 422;
}

export class VideoSourceNotFoundError extends AppError {
  readonly code = 'VIDEO_SOURCE_NOT_FOUND';
  readonly statusCode = 404;
}

export class NetworkError extends AppError {
  readonly code = 'NETWORK_ERROR';
  readonly statusCode = 503;
}

export class DownloadFailedError extends AppError {
  readonly code = 'DOWNLOAD_FAILED';
  readonly statusCode = 502;
}
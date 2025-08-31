import { Type, Static } from '@sinclair/typebox';

export const NameRequestSchema = Type.Object({
  url: Type.String({ format: 'uri', description: 'Video URL to resolve name for' }),
}, { 
  $id: 'NameRequest',
  title: 'Name Request',
  description: 'Request to resolve video name from URL'
});

export const NameResponseSchema = Type.Object({
  name: Type.String({ description: 'Resolved video name' }),
}, {
  $id: 'NameResponse',
  title: 'Name Response',
  description: 'Response containing resolved video name'
});

export const DownloadRequestSchema = Type.Object({
  url: Type.String({ format: 'uri', description: 'Video URL to download' }),
  name: Type.String({ description: 'Name to save the video as' }),
}, {
  $id: 'DownloadRequest',
  title: 'Download Request',
  description: 'Request to download video with specified name'
});

export const DownloadResponseSchema = Type.Object({
  jobId: Type.String({ description: 'Unique job identifier' }),
  status: Type.Literal('enqueued', { description: 'Download job status' }),
}, {
  $id: 'DownloadResponse',
  title: 'Download Response',
  description: 'Response with job ID for queued download'
});

export const CompletedDownloadSchema = Type.Object({
  url: Type.String({ format: 'uri', description: 'Original video URL' }),
  name: Type.String({ description: 'Video name' }),
  savedPath: Type.String({ description: 'Local file path where video was saved' }),
  size: Type.Number({ description: 'File size in bytes' }),
  startedAt: Type.String({ format: 'date-time', description: 'Download start timestamp' }),
  finishedAt: Type.String({ format: 'date-time', description: 'Download completion timestamp' }),
}, {
  $id: 'CompletedDownload',
  title: 'Completed Download',
  description: 'Information about a completed download'
});

export const CompletedDownloadsResponseSchema = Type.Object({
  downloads: Type.Array(CompletedDownloadSchema, { description: 'List of completed downloads' }),
}, {
  $id: 'CompletedDownloadsResponse',
  title: 'Completed Downloads Response',
  description: 'List of all completed downloads'
});

export const DownloadProgressSchema = Type.Object({
  jobId: Type.String({ description: 'Unique job identifier' }),
  url: Type.String({ format: 'uri', description: 'Video URL being downloaded' }),
  name: Type.String({ description: 'Video name' }),
  status: Type.Union([
    Type.Literal('downloading'),
    Type.Literal('processing')
  ], { description: 'Current download status' }),
  progressPercent: Type.Number({ minimum: 0, maximum: 100, description: 'Download progress percentage' }),
  downloadedBytes: Type.Number({ minimum: 0, description: 'Bytes downloaded so far' }),
  totalBytes: Type.Optional(Type.Number({ minimum: 0, description: 'Total file size in bytes if known' })),
  lastUpdated: Type.String({ format: 'date-time', description: 'Last progress update timestamp' }),
  startedAt: Type.String({ format: 'date-time', description: 'Download start timestamp' }),
}, {
  $id: 'DownloadProgress',
  title: 'Download Progress',
  description: 'Current progress information for an active download'
});

export const ProgressResponseSchema = Type.Object({
  progress: DownloadProgressSchema,
}, {
  $id: 'ProgressResponse',
  title: 'Progress Response',
  description: 'Response containing download progress information'
});

export const AllProgressResponseSchema = Type.Object({
  downloads: Type.Array(DownloadProgressSchema, { description: 'List of downloads in progress' }),
}, {
  $id: 'AllProgressResponse',
  title: 'All Progress Response',
  description: 'List of all downloads currently in progress'
});

export const HealthResponseSchema = Type.Object({
  status: Type.Literal('ok', { description: 'Service health status' }),
  timestamp: Type.String({ format: 'date-time', description: 'Current server timestamp' }),
}, {
  $id: 'HealthResponse',
  title: 'Health Response',
  description: 'Server health check response'
});

export type NameRequest = Static<typeof NameRequestSchema>;
export type NameResponse = Static<typeof NameResponseSchema>;
export type DownloadRequest = Static<typeof DownloadRequestSchema>;
export type DownloadResponse = Static<typeof DownloadResponseSchema>;
export type CompletedDownload = Static<typeof CompletedDownloadSchema>;
export type CompletedDownloadsResponse = Static<typeof CompletedDownloadsResponseSchema>;
export type DownloadProgress = Static<typeof DownloadProgressSchema>;
export type ProgressResponse = Static<typeof ProgressResponseSchema>;
export type AllProgressResponse = Static<typeof AllProgressResponseSchema>;
export type HealthResponse = Static<typeof HealthResponseSchema>;
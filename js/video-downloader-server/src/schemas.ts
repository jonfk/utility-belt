import { Type, Static } from '@sinclair/typebox';

export const NameRequestSchema = Type.Object({
  url: Type.String({ format: 'uri' }),
});

export const NameResponseSchema = Type.Object({
  name: Type.String(),
});

export const DownloadRequestSchema = Type.Object({
  url: Type.String({ format: 'uri' }),
  name: Type.String(),
});

export const DownloadResponseSchema = Type.Object({
  jobId: Type.String(),
  status: Type.Literal('enqueued'),
});

export const CompletedDownloadSchema = Type.Object({
  url: Type.String({ format: 'uri' }),
  name: Type.String(),
  savedPath: Type.String(),
  size: Type.Number(),
  startedAt: Type.String({ format: 'date-time' }),
  finishedAt: Type.String({ format: 'date-time' }),
});

export const CompletedDownloadsResponseSchema = Type.Array(CompletedDownloadSchema);

export const HealthResponseSchema = Type.Object({
  status: Type.Literal('ok'),
  timestamp: Type.String({ format: 'date-time' }),
});

export type NameRequest = Static<typeof NameRequestSchema>;
export type NameResponse = Static<typeof NameResponseSchema>;
export type DownloadRequest = Static<typeof DownloadRequestSchema>;
export type DownloadResponse = Static<typeof DownloadResponseSchema>;
export type CompletedDownload = Static<typeof CompletedDownloadSchema>;
export type CompletedDownloadsResponse = Static<typeof CompletedDownloadsResponseSchema>;
export type HealthResponse = Static<typeof HealthResponseSchema>;
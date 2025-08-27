import { FastifyInstance } from 'fastify';
import {
  NameRequestSchema,
  NameResponseSchema,
  DownloadRequestSchema,
  DownloadResponseSchema,
  CompletedDownloadsResponseSchema,
  HealthResponseSchema,
  type NameRequest,
  type NameResponse,
  type DownloadRequest,
  type DownloadResponse,
  type CompletedDownloadsResponse,
  type HealthResponse,
} from './schemas.js';

export async function registerRoutes(fastify: FastifyInstance) {
  fastify.post<{
    Body: NameRequest;
    Reply: NameResponse;
  }>('/v1/name', {
    schema: {
      body: NameRequestSchema,
      response: {
        200: NameResponseSchema,
      },
    },
  }, async (request, reply) => {
    reply.code(501);
    throw fastify.httpErrors.notImplemented('Name resolution not implemented');
  });

  fastify.post<{
    Body: DownloadRequest;
    Reply: DownloadResponse;
  }>('/v1/download', {
    schema: {
      body: DownloadRequestSchema,
      response: {
        200: DownloadResponseSchema,
      },
    },
  }, async (request, reply) => {
    reply.code(501);
    throw fastify.httpErrors.notImplemented('Download not implemented');
  });

  fastify.get<{
    Reply: CompletedDownloadsResponse;
  }>('/v1/downloads/completed', {
    schema: {
      response: {
        200: CompletedDownloadsResponseSchema,
      },
    },
  }, async (request, reply) => {
    reply.code(501);
    throw fastify.httpErrors.notImplemented('Downloads list not implemented');
  });

  fastify.get<{
    Reply: HealthResponse;
  }>('/healthz', {
    schema: {
      response: {
        200: HealthResponseSchema,
      },
    },
  }, async (request, reply) => {
    return {
      status: 'ok' as const,
      timestamp: new Date().toISOString(),
    };
  });
}
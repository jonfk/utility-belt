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
import { DefaultNameResolver } from './services/name.js';
import { DownloadService } from './services/download.js';

export async function registerRoutes(fastify: FastifyInstance) {
	const nameResolver = new DefaultNameResolver();
	const downloadService = new DownloadService();
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
		const { url } = request.body;
		const name = await nameResolver.resolveName(url);
		return { name };
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
		const { url, name } = request.body;
		const jobId = downloadService.enqueue(url, name);
		return { jobId, status: 'enqueued' };
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
		const downloads = downloadService.getCompleted();
		return { downloads };
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

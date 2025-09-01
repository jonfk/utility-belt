import { FastifyInstance } from 'fastify';
import {
	NameRequestSchema,
	NameResponseSchema,
	DownloadRequestSchema,
	DownloadResponseSchema,
	CompletedDownloadsResponseSchema,
	ProgressResponseSchema,
	AllProgressResponseSchema,
	HealthResponseSchema,
	type NameRequest,
	type NameResponse,
	type DownloadRequest,
	type DownloadResponse,
	type CompletedDownloadsResponse,
	type ProgressResponse,
	type AllProgressResponse,
	type HealthResponse,
} from './schemas.js';
import { DefaultNameResolver } from './services/name.js';
import { DownloadService } from './services/download.js';

export async function registerRoutes(fastify: FastifyInstance) {
	const nameResolver = new DefaultNameResolver();
	const downloadService = new DownloadService(fastify.log);
	fastify.post<{
		Body: NameRequest;
		Reply: NameResponse;
	}>('/api/name', {
		schema: {
			tags: ['Video'],
			summary: 'Resolve video name',
			description: 'Extract and resolve the name of a video from its URL',
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
	}>('/api/download', {
		schema: {
			tags: ['Video'],
			summary: 'Enqueue video download',
			description: 'Add a video download job to the queue',
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
	}>('/api/downloads/completed', {
		schema: {
			tags: ['Downloads'],
			summary: 'Get completed downloads',
			description: 'Retrieve list of all completed video downloads',
			response: {
				200: CompletedDownloadsResponseSchema,
			},
		},
	}, async (request, reply) => {
		const downloads = downloadService.getCompleted();
		return { downloads };
	});

	fastify.get<{
		Params: { jobId: string };
	}>('/api/downloads/progress/:jobId', {
		schema: {
			tags: ['Downloads'],
			summary: 'Get download progress',
			description: 'Retrieve progress information for a specific download job',
			params: {
				type: 'object',
				properties: {
					jobId: { type: 'string', description: 'Job ID to get progress for' }
				},
				required: ['jobId']
			},
			response: {
				200: ProgressResponseSchema,
				404: {
					type: 'object',
					properties: {
						error: { type: 'string' }
					}
				}
			},
		},
	}, async (request, reply) => {
		const progress = downloadService.getProgress(request.params.jobId);
		if (!progress) {
			reply.code(404);
			return { error: 'Download job not found or not in progress' };
		}
		return { progress };
	});

	fastify.get<{
		Reply: AllProgressResponse;
	}>('/api/downloads/progress', {
		schema: {
			tags: ['Downloads'],
			summary: 'Get all download progress',
			description: 'Retrieve progress information for all downloads currently in progress',
			response: {
				200: AllProgressResponseSchema,
			},
		},
	}, async (request, reply) => {
		const downloads = downloadService.getAllProgress();
		return { downloads };
	});

	fastify.get<{
		Reply: HealthResponse;
	}>('/healthz', {
		schema: {
			tags: ['Health'],
			summary: 'Health check',
			description: 'Check server health status',
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

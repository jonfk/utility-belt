import { FastifyInstance, FastifyError, FastifyRequest, FastifyReply } from 'fastify';
import { AppError } from './errors.js';

export async function registerErrorHandler(fastify: FastifyInstance) {
  fastify.setErrorHandler(async (error: FastifyError, request: FastifyRequest, reply: FastifyReply) => {
    // Log all errors with stack traces for debugging
    request.log.error({ err: error, req: request }, 'Request error occurred');

    // Handle validation errors (Fastify sets statusCode = 400)
    if (error.statusCode === 400 && error.validation) {
      return reply.status(400).send({
        code: 'VALIDATION_ERROR',
        message: 'Invalid request data',
        statusCode: 400,
        details: { validation: error.validation }
      });
    }

    // Handle typed domain errors
    if (error instanceof AppError) {
      return reply.status(error.statusCode).send({
        code: error.code,
        message: error.message,
        statusCode: error.statusCode,
        ...(error.details && { details: error.details })
      });
    }

    // Handle other HTTP errors (from @fastify/sensible or manually set statusCode)
    if (error.statusCode && error.statusCode >= 400 && error.statusCode < 500) {
      return reply.status(error.statusCode).send({
        code: 'CLIENT_ERROR',
        message: error.message,
        statusCode: error.statusCode
      });
    }

    // Handle unknown/server errors - never leak stack traces
    return reply.status(500).send({
      code: 'INTERNAL_ERROR',
      message: 'An internal server error occurred',
      statusCode: 500
    });
  });

  // Handle 404s separately (not passed through error handler)
  fastify.setNotFoundHandler(async (request: FastifyRequest, reply: FastifyReply) => {
    return reply.status(404).send({
      code: 'NOT_FOUND',
      message: 'Route not found',
      statusCode: 404
    });
  });
}
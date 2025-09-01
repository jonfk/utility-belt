import Fastify from 'fastify';
import { TypeBoxTypeProvider } from '@fastify/type-provider-typebox';
import sensible from '@fastify/sensible';
import swagger from '@fastify/swagger';
import swaggerUi from '@fastify/swagger-ui';
import staticPlugin from '@fastify/static';
import path from 'path';
import { fileURLToPath } from 'url';
import { config } from './config.js';
import { registerRoutes } from './routes.js';
import { ensureDataDir } from './storage.js';
import { closeBrowser } from './puppeteer.js';
import { registerErrorHandler } from './error-handler.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const fastify = Fastify({
  logger: true,
}).withTypeProvider<TypeBoxTypeProvider>();

async function start() {
  try {
    await fastify.register(sensible);
    await registerErrorHandler(fastify);

    await fastify.register(staticPlugin, {
      root: path.join(__dirname, '..', 'public'),
      prefix: '/',
    });

    await fastify.register(swagger, {
      openapi: {
        info: {
          title: 'Video Downloader Server',
          description: 'Simple utility service for downloading videos',
          version: '1.0.0',
        },
        servers: [
          {
            url: 'http://localhost:3000',
            description: 'Development server',
          },
        ],
      },
    });

    await fastify.register(swaggerUi, {
      routePrefix: '/docs',
      uiConfig: {
        docExpansion: 'list',
        deepLinking: false,
      },
    });

    await registerRoutes(fastify);
    await ensureDataDir();

    fastify.addHook('onClose', async () => {
      await closeBrowser();
    });

    await fastify.listen({
      port: config.port,
      host: '0.0.0.0',
    });

    console.log(`Server started on port ${config.port}`);
    console.log(`Data directory: ${config.dataDir}`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
}

process.on('SIGINT', async () => {
  console.log('Received SIGINT, gracefully shutting down...');
  await fastify.close();
});

process.on('SIGTERM', async () => {
  console.log('Received SIGTERM, gracefully shutting down...');
  await fastify.close();
});

start();
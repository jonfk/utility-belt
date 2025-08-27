import Fastify from 'fastify';
import { TypeBoxTypeProvider } from '@fastify/type-provider-typebox';
import sensible from '@fastify/sensible';
import { config } from './config.js';
import { registerRoutes } from './routes.js';
import { ensureDataDir } from './storage.js';
import { closeBrowser } from './puppeteer.js';

const fastify = Fastify({
  logger: true,
}).withTypeProvider<TypeBoxTypeProvider>();

async function start() {
  try {
    await fastify.register(sensible);
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
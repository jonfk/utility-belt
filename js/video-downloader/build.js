const { build } = require('esbuild');
const { chmod } = require('fs/promises');

async function bundle() {
    await build({
        entryPoints: ['video-downloader.js'],
        bundle: true,
        platform: 'node',
        target: 'node16',
        outfile: 'dist/video-downloader',
        banner: {
            js: '#!/usr/bin/env node',
        },
    });
    
    // Make the output file executable
    await chmod('dist/video-downloader', 0o755);
}

bundle().catch(console.error);

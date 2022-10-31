const major = process.versions.node.split('.')[0];

// @vercel/nft doe not support Node.js v8
if (major < 10) {
  process.exit(0);
}

// eslint-disable-next-line import/no-extraneous-dependencies
const { nodeFileTrace } = require('@vercel/nft');

const entryPoint = require.resolve('..');

// Trace the module entrypoint
nodeFileTrace([entryPoint]).then(result => {
  // eslint-disable-next-line no-console
  console.log('@vercel/nft traced dependencies:', Array.from(result.fileList));

  // If either binary is picked up, fail the test
  if (result.fileList.has('sentry-cli') || result.fileList.has('sentry-cli.exe')) {
    // eslint-disable-next-line no-console
    console.error('ERROR: The sentry-cli binary should not be found by @vercel/nft');
    process.exit(-1);
  } else {
    // eslint-disable-next-line no-console
    console.log('The sentry-cli binary was not traced by @vercel/nft');
  }
});

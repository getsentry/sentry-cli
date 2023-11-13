#!/usr/bin/env node

'use strict';

const http = require('http');
const fs = require('fs');
const path = require('path');
const { downloadBinary } = require('../js/install');

if (process.env.SENTRYCLI_LOCAL_CDNURL) {
  // For testing, mock the CDN by spawning a local server
  const server = http.createServer((request, response) => {
    const contents = fs.readFileSync(path.join(__dirname, '../js/__mocks__/sentry-cli'));
    response.writeHead(200, {
      'Content-Type': 'application/octet-stream',
      'Content-Length': String(contents.byteLength),
    });
    response.end(contents);
  });

  server.listen(8999);
  process.on('exit', () => server.close());
}

downloadBinary()
  .then(() => process.exit(0))
  .catch(e => {
    // eslint-disable-next-line no-console
    console.error(e.toString());
    process.exit(1);
  });

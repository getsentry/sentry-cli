const path = require('path');

process.env.SENTRY_BINARY_PATH = path.join(__dirname, 'lib', '__mocks__', 'sentry-cli');

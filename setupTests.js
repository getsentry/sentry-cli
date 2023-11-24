const path = require('path');

process.env.SENTRY_BINARY_PATH = path.join(__dirname, 'js', '__mocks__', 'sentry-cli');

'use strict';

import { format } from 'node:util';

export class Logger {
  constructor(public stream: NodeJS.WriteStream) {}

  log() {
    const message = format(...arguments);
    this.stream.write(`[sentry-cli] ${message}\n`);
  }
};

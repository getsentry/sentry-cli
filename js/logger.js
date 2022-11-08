class Logger {
  info() {
    this.stream.write(Array.from(arguments).join(' '));
  }
}

module.exports = new Logger();

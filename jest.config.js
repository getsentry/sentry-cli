module.exports = {
  setupFiles: ['<rootDir>/setupTests.js'],
  testPathIgnorePatterns: ['<rootDir>/src/'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
};

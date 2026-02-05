# JavaScript Development Guidelines

## Code Organization

- Maintain compatibility with existing npm package structure
- Consider impact on installation flow in `scripts/install.js`
- Test across different Node.js versions

## Installation & Distribution

- Installation logic in `scripts/install.js` handles platform detection
- Consider offline/air-gapped environments
- Binary management via `npm-binary-distributions/`

## Development Commands

```bash
# JavaScript workflow
npm test
npm run fix
npm run check:types
```

## Code Quality

- Uses ESLint, Prettier, and Jest
- Follow existing patterns for error handling
- Maintain backward compatibility

## TypeScript Support

- Type definitions are generated via TypeScript
- Sync with Rust CLI interface changes
- Consider backward compatibility for JS API

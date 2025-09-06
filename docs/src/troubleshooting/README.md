# Troubleshooting

Common issues and solutions for Gold Digger.

## Quick Diagnostics

Before diving into specific issues, try these basic checks:

1. **Verify Installation**: `gold_digger --version`
2. **Check Configuration**: Ensure all required parameters are set
3. **Test Database Connection**: Use a simple query first
4. **Review Error Messages**: Look for specific error codes and messages

## Common Issue Categories

- [Connection Problems](connection-issues.md) - Database connectivity and TLS issues (rustls-only
  implementation)
- [Type Safety](type-safety.md) - Safe data type handling and conversion
- [Type Errors](type-errors.md) - Data type conversion problems
- [Performance Issues](performance.md) - Slow queries and memory usage

## Getting Help

If you can't find a solution here:

1. Check the [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues)
2. Review the [Configuration Guide](../usage/configuration.md)
3. Create a new issue with detailed error information

## Error Codes

Gold Digger uses standard exit codes:

- `0`: Success
- `1`: No results returned
- `2`: Configuration error
- `3`: Database connection failure
- `4`: Query execution failure
- `5`: File I/O error

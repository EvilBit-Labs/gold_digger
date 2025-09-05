# Integration Testing Issues

Common issues and solutions for Gold Digger's integration testing framework.

## Docker-Related Issues

### Docker Not Available

**Symptom**: Integration tests fail with "Docker not available" or similar errors.

**Solution**:

```bash
# Check Docker status
docker info

# Start Docker service (Linux)
sudo systemctl start docker

# Start Docker Desktop (macOS/Windows)
# Use Docker Desktop application

# Verify Docker is working
docker run hello-world
```

### Container Startup Timeouts

**Symptom**: Tests fail with container startup timeouts in CI environments.

**Solution**:

```bash
# Increase timeout for CI environments
export TESTCONTAINERS_WAIT_TIMEOUT=300

# Run tests with extended timeout
cargo test --features integration_tests -- --ignored
```

### Port Conflicts

**Symptom**: Tests fail with "port already in use" errors.

**Solution**:

```bash
# Check for conflicting processes
lsof -i :3306

# Kill conflicting MySQL/MariaDB processes
sudo pkill -f mysql
sudo pkill -f mariadb

# Use random ports (testcontainers default)
# No manual port configuration needed
```

## Test Execution Issues

### Integration Tests Not Running

**Symptom**: Integration tests are skipped or not executed.

**Solution**:

```bash
# Ensure integration_tests feature is enabled
cargo test --features integration_tests -- --ignored

# Check test discovery
cargo test --features integration_tests --list | grep integration

# Run specific integration test
cargo test --features integration_tests test_mysql_connection -- --ignored
```

### Test Data Issues

**Symptom**: Tests fail due to missing or incorrect test data.

**Solution**:

```bash
# Verify test fixtures exist
ls tests/fixtures/

# Check schema and seed data
cat tests/fixtures/schema.sql
cat tests/fixtures/seed_data.sql

# Regenerate test data if needed
just test-integration --verbose
```

### Memory Issues with Large Datasets

**Symptom**: Tests fail with out-of-memory errors during large dataset testing.

**Solution**:

```bash
# Increase available memory for tests
export RUST_MIN_STACK=8388608

# Run tests with memory profiling
cargo test --features integration_tests --release -- --ignored

# Skip performance tests if memory-constrained
cargo test --features integration_tests -- --ignored --skip performance
```

## TLS Certificate Issues

### Certificate Validation Failures

**Symptom**: TLS tests fail with certificate validation errors.

**Solution**:

```bash
# Check TLS certificate fixtures
ls tests/fixtures/tls/

# Verify certificate format
openssl x509 -in tests/fixtures/tls/server.pem -text -noout

# Regenerate certificates if expired
# (Certificate generation scripts in tests/fixtures/tls/)
```

### TLS Connection Failures

**Symptom**: TLS integration tests fail to establish secure connections.

**Solution**:

```bash
# Test TLS configuration separately
cargo test --test tls_config_unit_tests

# Check rustls dependencies
cargo tree | grep rustls

# Verify no native-tls conflicts
! cargo tree | grep native-tls || echo "native-tls conflict detected"
```

## CI Environment Issues

### GitHub Actions Failures

**Symptom**: Integration tests pass locally but fail in GitHub Actions.

**Solution**:

```bash
# Test with act (local GitHub Actions simulation)
act -j test-integration

# Check CI-specific environment variables
env | grep CI
env | grep GITHUB

# Use CI-compatible timeouts
export CI=true
export GITHUB_ACTIONS=true
```

### Resource Limits in CI

**Symptom**: Tests fail due to CI resource constraints.

**Solution**:

```bash
# Use smaller test datasets in CI
if [ "$CI" = "true" ]; then
  export TEST_DATASET_SIZE=100
else
  export TEST_DATASET_SIZE=1000
fi

# Skip resource-intensive tests in CI
cargo test --features integration_tests -- --ignored --skip large_dataset
```

## Database-Specific Issues

### MySQL Version Compatibility

**Symptom**: Tests fail with MySQL version-specific errors.

**Solution**:

```bash
# Check MySQL version in container
docker exec -it <container_id> mysql --version

# Use specific MySQL version
# (Configured in testcontainers setup)

# Test with multiple MySQL versions
cargo test --features integration_tests mysql_8_0 -- --ignored
cargo test --features integration_tests mysql_8_1 -- --ignored
```

### MariaDB SSL Configuration

**Symptom**: MariaDB TLS tests fail with SSL configuration errors.

**Solution**:

```bash
# Check MariaDB SSL status
docker exec -it <container_id> mysql -e "SHOW VARIABLES LIKE 'have_ssl';"

# Verify SSL certificate mounting
docker exec -it <container_id> ls -la /etc/mysql/ssl/

# Check MariaDB error log
docker logs <container_id>
```

## Performance Issues

### Slow Test Execution

**Symptom**: Integration tests take too long to complete.

**Solution**:

```bash
# Use nextest for parallel execution
cargo nextest run --features integration_tests -- --ignored

# Run specific test categories
cargo test --features integration_tests data_types -- --ignored

# Skip performance tests for faster feedback
cargo test --features integration_tests -- --ignored --skip performance
```

### Container Cleanup Issues

**Symptom**: Containers not cleaned up after tests, consuming resources.

**Solution**:

```bash
# Enable Ryuk for automatic cleanup
export TESTCONTAINERS_RYUK_DISABLED=false

# Manual container cleanup
docker ps -a | grep testcontainers | awk '{print $1}' | xargs docker rm -f

# Clean up test networks
docker network prune -f
```

## Debugging Integration Tests

### Verbose Test Output

```bash
# Run with verbose output
RUST_LOG=debug cargo test --features integration_tests -- --ignored --nocapture

# Keep containers running for inspection
TESTCONTAINERS_RYUK_DISABLED=true cargo test --features integration_tests -- --ignored

# Generate detailed test reports
cargo nextest run --features integration_tests --profile ci -- --ignored
```

### Container Inspection

```bash
# List running containers
docker ps

# Inspect container configuration
docker inspect <container_id>

# Access container shell
docker exec -it <container_id> /bin/bash

# Check container logs
docker logs <container_id>
```

### Test Coverage Analysis

```bash
# Generate coverage for integration tests
cargo llvm-cov --features integration_tests --html -- --ignored

# View coverage report
open target/llvm-cov/html/index.html
```

## Getting Help

If you encounter issues not covered here:

1. **Check the logs**: Enable verbose logging with `RUST_LOG=debug`
2. **Verify Docker setup**: Ensure Docker is running and accessible
3. **Test isolation**: Run individual tests to isolate issues
4. **CI reproduction**: Use `act` to reproduce CI failures locally
5. **Create an issue**: Report bugs with detailed error messages and environment information

For more information, see the [Integration Testing Framework](../development/integration-testing.md) documentation.

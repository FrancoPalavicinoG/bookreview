# BookReview Load Testing

This directory contains JMeter load tests and monitoring scripts for the BookReview application. The tests compare performance between multiple deployment modes:

1. **Basic Deployment**: Rust Web Application + MongoDB
2. **Cache Deployment**: Rust Web Application + MongoDB + Redis Cache
3. **Proxy Deployment**: Apache Reverse Proxy + Rust Web Application + MongoDB (legacy)
4. **Production Deployment**: Apache Reverse Proxy + Rust Web Application + MongoDB + Redis Cache

## Prerequisites

### Install JMeter

**macOS (Homebrew):**
```bash
brew install openjdk@11
brew install jmeter
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install openjdk-11-jdk
wget https://archive.apache.org/dist/jmeter/binaries/apache-jmeter-5.5.tgz
tar -xzf apache-jmeter-5.5.tgz
sudo mv apache-jmeter-5.5 /opt/jmeter
echo 'export PATH="/opt/jmeter/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**Verify Installation:**
```bash
jmeter -v
```

### Other Requirements

- Docker and Docker Compose
- `curl` command
- `bc` calculator (usually pre-installed)
- Basic Unix tools (`awk`, `grep`, `sed`)

## Test Configuration

### Load Levels
- **1 user** - Baseline performance
- **10 users** - Light load
- **100 users** - Medium load  
- **1000 users** - Heavy load
- **5000 users** - Stress test

### Test Duration
- **5 minutes** per test scenario

### Endpoints Tested
- `GET /health` - Health check endpoint
- `GET /` - Home page
- `GET /books` - Books listing
- `GET /authors` - Authors listing  
- `GET /reviews` - Reviews listing
- `GET /sales` - Sales listing
- `GET /search?q=book` - Search functionality
- `GET /static/*` - Static file serving (proxy mode only)

## Quick Start

### Run All Redis-enabled Tests (Recommended)

To run comprehensive load tests for both cache and production deployments with Redis:

```bash
cd load-testing
./run-redis-load-tests.sh
```

This will test both:
- **Cache Deployment**: App + Database + Redis (port 8000)
- **Production Deployment**: Proxy + App + Database + Redis (port 80)

### Run Specific Redis Deployment Tests

```bash
# Test only cache deployment (app + database + redis)
./run-redis-load-tests.sh cache

# Test only production deployment (proxy + app + database + redis)  
./run-redis-load-tests.sh production
```

### Legacy Tests (Without Redis)

For testing basic and proxy deployments without Redis:

```bash
# Run legacy tests (basic and proxy without Redis)
./run-load-tests.sh
```

## Quick Start

### 1. Setup Deployments

**Basic Deployment:**
```bash
# From project root
docker compose -f docker-compose.basic.yml up -d --build

# Load sample data
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# Verify it's working
curl http://localhost:8000/health
```

**Proxy Deployment:**
```bash
# Add to /etc/hosts (if not already done)
echo "127.0.0.1 app.localhost" | sudo tee -a /etc/hosts

# From project root  
docker compose up -d --build

# Load sample data
docker compose run --rm web sh -lc '/app/seeder'

# Verify it's working
curl http://app.localhost/health
```

### 2. Run Individual Tests

**Quick Test (Manual):**
```bash
cd load-testing

# Test basic deployment with 10 users
./simple-test.sh basic 10

# Test proxy deployment with 100 users  
./simple-test.sh proxy 100
```

**Collect System Metrics:**
```bash
# Monitor basic deployment for 5 minutes
./collect-metrics.sh basic 300

# Monitor proxy deployment for 10 minutes
./collect-metrics.sh proxy 600
```

### 3. Run Complete Test Suite

```bash
# This will run all tests automatically (takes ~50 minutes)
./run-load-tests.sh
```

## Manual Testing Workflow

For more control, run tests manually:

### Step 1: Start Basic Deployment
```bash
cd ..  # Go to project root
docker compose -f docker-compose.basic.yml up -d --build
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'
```

### Step 2: Test Basic Deployment
```bash
cd load-testing

# Start metrics collection in background
./collect-metrics.sh basic 300 &
METRICS_PID=$!

# Run JMeter test with 1 user
./simple-test.sh basic 1

# Wait for metrics collection to finish
wait $METRICS_PID

# Repeat for other user counts: 10, 100, 1000, 5000
```

### Step 3: Switch to Proxy Deployment
```bash
cd ..
docker compose -f docker-compose.basic.yml down
docker compose up -d --build
docker compose run --rm web sh -lc '/app/seeder'
```

### Step 4: Test Proxy Deployment
```bash
cd load-testing

# Repeat the same process for proxy deployment
./collect-metrics.sh proxy 300 &
METRICS_PID=$!
./simple-test.sh proxy 1
wait $METRICS_PID

# Continue with other user counts...
```

## File Structure

```
load-testing/
├── README.md                          # This file
├── basic-deployment-test.jmx           # JMeter test plan for basic deployment
├── proxy-deployment-test.jmx           # JMeter test plan for production deployment (proxy + app + db + redis)
├── cache-deployment-test.jmx           # JMeter test plan for cache deployment (app + db + redis)
├── run-load-tests.sh                   # Legacy automated test suite (without Redis)
├── run-redis-load-tests.sh             # New automated test suite (with Redis)
├── simple-test.sh                      # Run individual JMeter tests
├── collect-metrics.sh                  # Collect Docker container metrics
└── results/                            # Generated test results
    ├── cache/                          # Cache deployment results (app + db + redis)
    │   ├── cache_1users_metrics.txt    # System metrics with CPU, memory, threads
    │   ├── cache_1users_jmeter.jtl     # JMeter results files
    │   ├── cache_1users_report/        # JMeter HTML reports
    │   ├── cache_1users_summary.txt    # Test summary statistics
    │   └── ... (files for 10, 100, 1000, 5000 users)
    ├── production/                     # Production deployment results (proxy + app + db + redis)
    │   ├── production_1users_metrics.txt  # System metrics with CPU, memory, threads
    │   ├── production_1users_jmeter.jtl   # JMeter results files
    │   ├── production_1users_report/      # JMeter HTML reports
    │   ├── production_1users_summary.txt  # Test summary statistics
    │   └── ... (files for 10, 100, 1000, 5000 users)
    ├── basic/                          # Legacy: Basic deployment results (app + db only)
    │   └── basic_*users_*              # Legacy test files
    ├── basic-proxy/                    # Legacy: Proxy deployment results (proxy + app + db only)
    │   └── proxy_*users_*              # Legacy test files
    └── load_test_report.md             # Summary report
```

## Metrics Collected

### Application Metrics (JMeter)
- **Response Times**: Min, Max, Average, 90th percentile, 95th percentile
- **Throughput**: Requests per second
- **Error Rate**: Percentage of failed requests
- **Response Codes**: HTTP status code distribution

### System Metrics (Docker Stats)
Collected every 5 seconds for each container:
- **CPU Usage**: Percentage per container
- **Memory Usage**: MB and percentage per container  
- **Thread Count**: Number of threads/processes per container
  - Rust app: Process threads
  - Apache: Process count
  - MongoDB: Connection count
  - Redis: Connected clients count

## Understanding Results

### JMeter Results (.jtl files)
Comma-separated files with detailed timing data for each request:
- `timestamp` - When the request was made
- `elapsed` - Response time in milliseconds
- `label` - Endpoint name
- `responseCode` - HTTP status code
- `success` - true/false for request success
- `bytes` - Response size in bytes

### JMeter HTML Reports
Interactive HTML dashboards showing:
- Response time trends over time
- Throughput over time
- Error rate analysis
- Response time percentiles
- Request distribution

### System Metrics (.csv files)
Container resource usage over time:
- `timestamp` - When metrics were collected
- `container_name` - Which container
- `cpu_percent` - CPU usage percentage
- `memory_usage_mb` - Memory usage in MB
- `memory_percent` - Memory usage percentage
- Additional network and disk I/O metrics

## Analysis Tips

### Performance Comparison
1. **Response Times**: Compare average and 95th percentile response times
2. **Throughput**: Higher requests/second = better performance
3. **Resource Usage**: Lower CPU/memory usage = more efficient
4. **Error Rates**: Should be 0% or very low for valid comparison

### Key Questions to Answer
1. How do response times compare between deployments?
2. Which deployment can handle more concurrent users?
3. What's the resource overhead of the reverse proxy?
4. At what point does each deployment start to degrade?
5. Are there any error patterns under high load?

### Expected Patterns
- **Basic Deployment**: Lower latency, direct connection, but may use more app server resources for static files
- **Proxy Deployment**: Potentially higher latency due to proxy hop, but better static file performance and caching

## Troubleshooting

### Common Issues

**JMeter not found:**
```bash
# Check installation
jmeter -v

# If not installed, install via Homebrew (macOS)
brew install jmeter
```

**Application not responding:**
```bash
# Check if containers are running
docker ps

# Check application logs
docker compose logs web

# Test manual connectivity
curl -v http://localhost:8000/health
curl -v http://app.localhost/health
```

**Permission denied on scripts:**
```bash
chmod +x *.sh
```

**Large result files:**
The JMeter result files can become quite large. To manage disk space:
```bash
# Compress old results
gzip results/*.jtl

# Clean up old HTML reports
rm -rf results/*_report_*
```

### Performance Tips

**For more accurate results:**
1. Close other applications during testing
2. Run tests multiple times and average results
3. Use a dedicated test machine if possible
4. Ensure Docker has adequate resources allocated

**To reduce test time:**
- Reduce test duration from 300s to 60s for initial runs
- Start with smaller user counts (1, 10, 50 instead of going up to 5000)
- Run only the essential endpoints

## Sample Commands Reference

```bash
# Basic deployment workflow
docker compose -f docker-compose.basic.yml up -d --build
./simple-test.sh basic 10
./collect-metrics.sh basic 300
docker compose -f docker-compose.basic.yml down

# Proxy deployment workflow  
docker compose up -d --build
./simple-test.sh proxy 10
./collect-metrics.sh proxy 300
docker compose down

# Check test results
ls -la results/
open results/basic_10users_*_report/index.html
```

This setup provides comprehensive load testing capabilities to analyze and compare the performance characteristics of both deployment architectures.

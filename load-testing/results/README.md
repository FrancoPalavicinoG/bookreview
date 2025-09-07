# Load Testing Results

This directory contains the results from JMeter load tests and system metrics collection.

## File Types

- `*.jtl` - JMeter raw results files
- `*_report/` - JMeter HTML report directories  
- `*_metrics_*.csv` - Docker container system metrics
- `*_jmeter.log` - JMeter execution logs
- `load_test_report.md` - Generated summary report

## File Naming Convention

- `basic_*` - Results from basic deployment (app + database)
- `proxy_*` - Results from proxy deployment (apache + app + database)
- `*_1users_*` - Results with 1 concurrent user
- `*_10users_*` - Results with 10 concurrent users
- etc.

Results are automatically generated when running the load testing scripts.

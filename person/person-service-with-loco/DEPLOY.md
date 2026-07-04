# Person Service - Deployment Guide

This guide covers deploying the Person Service using Podman and Podman Compose. (The compose files keep their
`docker-compose.yml` names, which `podman compose` reads as-is.)

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start (Development)](#quick-start-development)
- [Production Deployment](#production-deployment)
- [Testing Deployment](#testing-deployment)
- [Configuration](#configuration)
- [Database Migrations](#database-migrations)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

- **Podman**: Version 4.0 or later
- **Podman Compose**: the built-in `podman compose` subcommand

### Verify Installation

```bash
podman --version
podman compose --version
```

## Quick Start (Development)

### 1. Clone Repository

```bash
git clone https://github.com/SixArm/main-x-service.git
cd main-x-service/person/person-service-with-loco
```

### 2. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit configuration as needed
nano .env
```

### 3. Build and Start Services

```bash
# Build the person server image
podman compose build

# Start all services (PostgreSQL + Person Server)
podman compose up -d

# View logs
podman compose logs -f person-server
```

### 4. Run Database Migrations

```bash
# Access the person server container
podman compose exec person-server bash

# Inside the container, run migrations
sea-orm-cli migrate up --database-url=$DATABASE_URL

# Exit the container
exit
```

### 5. Verify Deployment

```bash
# Check service health
curl http://localhost:8080/api/health

# Expected response:
# {
#   "status": "healthy",
#   "service": "person-service",
#   "version": "0.1.0"
# }
```

### 6. Access Services

- **API**: http://localhost:8080/api
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050

To enable pgAdmin:

```bash
podman compose --profile tools up -d
```

## Production Deployment

### 1. Prepare Production Environment

```bash
# Copy production environment template
cp .env.production.example .env.production

# Edit with production values
nano .env.production
```

**IMPORTANT**: Update the following in `.env.production`:

- `DATABASE_URL` - Use strong password and SSL connection
- `POSTGRES_PASSWORD` - Use cryptographically strong password
- `RUST_LOG` - Set to `info` for production

### 2. Build Production Image

```bash
# Build with production optimizations
podman build -t person-server:latest .

# Tag for registry
podman tag person-server:latest your-registry.com/person_service-server:v1.0.0
```

### 3. Push to Container Registry

```bash
# Login to your container registry
podman login your-registry.com

# Push image
podman push your-registry.com/person_service-server:v1.0.0
```

### 4. Deploy to Production Server

```bash
# SSH to production server
ssh production-server

# Pull latest image
podman pull your-registry.com/person_service-server:v1.0.0

# Start with production compose file
podman compose --env-file .env.production up -d
```

### 5. Production Checklist

- [ ] Use SSL/TLS for database connections (`sslmode=require`)
- [ ] Use strong, unique passwords for all services
- [ ] Configure firewall rules (only expose necessary ports)
- [ ] Set up database backups
- [ ] Configure log aggregation
- [ ] Set up monitoring and alerting
- [ ] Use volume mounts for persistent data
- [ ] Enable container restart policies
- [ ] Configure resource limits (CPU, memory)
- [ ] Set up health checks

## Testing Deployment

Run the full test suite using Podman Compose:

```bash
# Build test image and run tests
podman compose -f docker-compose.test.yml up --build

# View test results
podman compose -f docker-compose.test.yml logs test-runner

# Clean up test containers
podman compose -f docker-compose.test.yml down -v
```

### Expected Test Output

```
Running unit tests...
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured

Running integration tests...
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured
```

## Configuration

### Environment Variables

All configuration is done via environment variables. See `.env.example` for complete list.

#### Database

```bash
DATABASE_URL=postgresql://user:password@host:5432/database
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=2
```

#### Server

```bash
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
```

#### Search Engine

```bash
SEARCH_INDEX_PATH=/app/data/search_index
```

#### Matching Algorithm

```bash
MATCHING_THRESHOLD=0.7
MATCHING_NAME_WEIGHT=0.4
MATCHING_DOB_WEIGHT=0.3
MATCHING_GENDER_WEIGHT=0.1
MATCHING_ADDRESS_WEIGHT=0.2
```

#### Logging

```bash
RUST_LOG=info
RUST_BACKTRACE=0
```

### Compose Profiles

#### Default Profile

Starts only essential services (PostgreSQL + Person Server):

```bash
podman compose up -d
```

#### Tools Profile

Includes pgAdmin for database management:

```bash
podman compose --profile tools up -d
```

## Database Migrations

### Running Migrations

#### Method 1: Inside Container

```bash
podman compose exec person-server bash
sea-orm-cli migrate up
exit
```

#### Method 2: Init Container (Recommended for Production)

Add to `docker-compose.yml`:

```yaml
person-migrations:
  image: person-server:latest
  depends_on:
    postgres:
      condition: service_healthy
  environment:
    DATABASE_URL: ${DATABASE_URL}
  command: sea-orm-cli migrate up
  networks:
    - person-network
```

Then:

```bash
podman compose up person-migrations
```

### Creating New Migrations

```bash
# Inside development environment
sea-orm-cli migrate generate add_new_feature

# Edit up.sql and down.sql
# Test migration
sea-orm-cli migrate up
sea-orm-cli migrate refresh
```

## Monitoring

### Health Checks

The person server includes a health check endpoint:

```bash
curl http://localhost:8080/api/health
```

### Container Health Checks

Health checks are configured in `docker-compose.yml`:

```bash
# View container health status
podman compose ps

# View health check logs
podman inspect person-server --format='{{json .State.Health}}'
```

### Logs

```bash
# View all logs
podman compose logs

# Follow logs
podman compose logs -f

# View specific service logs
podman compose logs person-server
podman compose logs postgres

# View last 100 lines
podman compose logs --tail=100 person-server
```

### Metrics

TODO: Implement Prometheus metrics endpoint

### Resource Usage

```bash
# View resource usage
podman stats

# View resource usage for specific container
podman stats person-server
```

## Troubleshooting

### Container Won't Start

**Check logs**:

```bash
podman compose logs person-server
```

**Common issues**:

- Database not ready: Wait for PostgreSQL health check
- Missing environment variables: Check `.env` file
- Port already in use: Change `PERSON_PORT` in `.env`

### Database Connection Issues

**Test database connectivity**:

```bash
podman compose exec postgres psql -U person_user -d person_service -c "SELECT 1;"
```

**Common issues**:

- Wrong credentials: Check `DATABASE_URL` matches PostgreSQL settings
- Network issues: Ensure containers are on same network
- PostgreSQL not ready: Check PostgreSQL health status

### Migration Failures

**Reset database** (CAUTION: Destroys all data):

```bash
podman compose down -v
podman compose up -d postgres
# Wait for PostgreSQL to be ready
podman compose exec postgres psql -U person_user -d person_service
# Inside psql:
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
\q
# Run migrations
podman compose exec person-server sea-orm-cli migrate up
```

### Search Index Issues

**Reset search index**:

```bash
podman compose exec person-server rm -rf /app/data/search_index/*
podman compose restart person-server
```

### High Memory Usage

**Adjust connection pool sizes**:

```bash
# In .env
DATABASE_MAX_CONNECTIONS=5
DATABASE_MIN_CONNECTIONS=1
```

**Set container memory limits**:

```yaml
# In docker-compose.yml
services:
  person-server:
    deploy:
      resources:
        limits:
          memory: 512M
```

### Port Conflicts

**Change exposed ports**:

```bash
# In .env
PERSON_PORT=8081
POSTGRES_PORT=5433
PGADMIN_PORT=5051
```

## Backup and Recovery

### Database Backup

```bash
# Create backup
podman compose exec postgres pg_dump -U person_user person_service > backup-$(date +%Y%m%d).sql

# Restore from backup
podman compose exec -T postgres psql -U person_user person_service < backup-20231228.sql
```

### Search Index Backup

```bash
# Backup search index
podman cp person-server:/app/data/search_index ./search_index_backup

# Restore search index
podman cp ./search_index_backup person-server:/app/data/search_index
podman compose restart person-server
```

## Security Best Practices

1. **Use Strong Passwords**: Generate cryptographically strong passwords
2. **Enable SSL**: Use SSL for database connections in production
3. **Limit Network Exposure**: Only expose necessary ports
4. **Regular Updates**: Keep container images and dependencies updated
5. **Secrets Management**: Use Podman secrets or environment variable injection
6. **Run as Non-Root**: Container runs as `person` user (UID 1000)
7. **Resource Limits**: Set memory and CPU limits in production
8. **Log Management**: Rotate logs and avoid logging sensitive data

## Performance Tuning

### Database Connection Pool

```bash
# Adjust based on workload
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
```

### Search Index

```bash
# Increase cache for better search performance
SEARCH_CACHE_SIZE_MB=2048
```

### Container Resources

```yaml
services:
  person-server:
    deploy:
      resources:
        limits:
          cpus: "2"
          memory: 1G
        reservations:
          cpus: "1"
          memory: 512M
```

## Scaling

### Horizontal Scaling

For high-availability deployments:

1. **Load Balancer**: Use nginx or HAProxy in front of multiple person-server instances
2. **Shared Database**: All instances connect to same PostgreSQL
3. **Shared Search Index**: Use network-mounted search index or separate search service
4. **Stateless Design**: the person server is stateless, scales horizontally

Example:

```bash
podman compose up -d --scale person-server=3
```

### Vertical Scaling

Increase resources for single instance:

```yaml
services:
  person-server:
    deploy:
      resources:
        limits:
          cpus: "4"
          memory: 4G
```

## Next Steps

- Set up CI/CD pipeline for automated deployments
- Configure monitoring with Prometheus and Grafana
- Implement authentication and authorization
- Set up log aggregation (ELK stack or similar)
- Configure automated backups
- Implement disaster recovery procedures

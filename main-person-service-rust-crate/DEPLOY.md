# Main Person Service - Deployment Guide

This guide covers deploying the Main Person Service (MPI) system using Docker and Docker Compose.

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

- **Docker**: Version 20.10 or later
- **Docker Compose**: Version 2.0 or later

### Verify Installation

```bash
docker --version
docker-compose --version
```

## Quick Start (Development)

### 1. Clone Repository

```bash
git clone https://github.com/your-org/main-person-service-rust-crate.git
cd main-person-service-rust-crate
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
# Build the MPI server image
docker-compose build

# Start all services (PostgreSQL + MPI Server)
docker-compose up -d

# View logs
docker-compose logs -f mpi-server
```

### 4. Run Database Migrations

```bash
# Access the MPI server container
docker-compose exec mpi-server bash

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
#   "service": "main-person-service",
#   "version": "0.1.0"
# }
```

### 6. Access Services

- **MPI API**: http://localhost:8080/api
- **Swagger UI**: http://localhost:8080/swagger-ui
- **pgAdmin** (optional): http://localhost:5050

To enable pgAdmin:

```bash
docker-compose --profile tools up -d
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
docker build -t mpi-server:latest .

# Tag for registry
docker tag mpi-server:latest your-registry.com/main_person_service-server:v1.0.0
```

### 3. Push to Container Registry

```bash
# Login to your container registry
docker login your-registry.com

# Push image
docker push your-registry.com/main_person_service-server:v1.0.0
```

### 4. Deploy to Production Server

```bash
# SSH to production server
ssh production-server

# Pull latest image
docker pull your-registry.com/main_person_service-server:v1.0.0

# Start with production compose file
docker-compose -f docker-compose.production.yml up -d
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

Run the full test suite using Docker Compose:

```bash
# Build test image and run tests
docker-compose -f docker-compose.test.yml up --build

# View test results
docker-compose -f docker-compose.test.yml logs test-runner

# Clean up test containers
docker-compose -f docker-compose.test.yml down -v
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

### Docker Compose Profiles

#### Default Profile

Starts only essential services (PostgreSQL + MPI Server):

```bash
docker-compose up -d
```

#### Tools Profile

Includes pgAdmin for database management:

```bash
docker-compose --profile tools up -d
```

## Database Migrations

### Running Migrations

#### Method 1: Inside Container

```bash
docker-compose exec mpi-server bash
sea-orm-cli migrate up
exit
```

#### Method 2: Init Container (Recommended for Production)

Add to `docker-compose.yml`:

```yaml
mpi-migrations:
  image: mpi-server:latest
  depends_on:
    postgres:
      condition: service_healthy
  environment:
    DATABASE_URL: ${DATABASE_URL}
  command: sea-orm-cli migrate up
  networks:
    - mpi-network
```

Then:

```bash
docker-compose up mpi-migrations
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

The MPI server includes a health check endpoint:

```bash
curl http://localhost:8080/api/health
```

### Docker Health Checks

Health checks are configured in `docker-compose.yml`:

```bash
# View container health status
docker-compose ps

# View health check logs
docker inspect mpi-server --format='{{json .State.Health}}'
```

### Logs

```bash
# View all logs
docker-compose logs

# Follow logs
docker-compose logs -f

# View specific service logs
docker-compose logs mpi-server
docker-compose logs postgres

# View last 100 lines
docker-compose logs --tail=100 mpi-server
```

### Metrics

TODO: Implement Prometheus metrics endpoint

### Resource Usage

```bash
# View resource usage
docker stats

# View resource usage for specific container
docker stats mpi-server
```

## Troubleshooting

### Container Won't Start

**Check logs**:

```bash
docker-compose logs mpi-server
```

**Common issues**:

- Database not ready: Wait for PostgreSQL health check
- Missing environment variables: Check `.env` file
- Port already in use: Change `MPI_PORT` in `.env`

### Database Connection Issues

**Test database connectivity**:

```bash
docker-compose exec postgres psql -U mpi_user -d mpi -c "SELECT 1;"
```

**Common issues**:

- Wrong credentials: Check `DATABASE_URL` matches PostgreSQL settings
- Network issues: Ensure containers are on same network
- PostgreSQL not ready: Check PostgreSQL health status

### Migration Failures

**Reset database** (CAUTION: Destroys all data):

```bash
docker-compose down -v
docker-compose up -d postgres
# Wait for PostgreSQL to be ready
docker-compose exec postgres psql -U mpi_user -d mpi
# Inside psql:
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
\q
# Run migrations
docker-compose exec mpi-server sea-orm-cli migrate up
```

### Search Index Issues

**Reset search index**:

```bash
docker-compose exec mpi-server rm -rf /app/data/search_index/*
docker-compose restart mpi-server
```

### High Memory Usage

**Adjust connection pool sizes**:

```bash
# In .env
DATABASE_MAX_CONNECTIONS=5
DATABASE_MIN_CONNECTIONS=1
```

**Set Docker memory limits**:

```yaml
# In docker-compose.yml
services:
  mpi-server:
    deploy:
      resources:
        limits:
          memory: 512M
```

### Port Conflicts

**Change exposed ports**:

```bash
# In .env
MPI_PORT=8081
POSTGRES_PORT=5433
PGADMIN_PORT=5051
```

## Backup and Recovery

### Database Backup

```bash
# Create backup
docker-compose exec postgres pg_dump -U mpi_user mpi > backup-$(date +%Y%m%d).sql

# Restore from backup
docker-compose exec -T postgres psql -U mpi_user mpi < backup-20231228.sql
```

### Search Index Backup

```bash
# Backup search index
docker cp mpi-server:/app/data/search_index ./search_index_backup

# Restore search index
docker cp ./search_index_backup mpi-server:/app/data/search_index
docker-compose restart mpi-server
```

## Security Best Practices

1. **Use Strong Passwords**: Generate cryptographically strong passwords
2. **Enable SSL**: Use SSL for database connections in production
3. **Limit Network Exposure**: Only expose necessary ports
4. **Regular Updates**: Keep Docker images and dependencies updated
5. **Secrets Management**: Use Docker secrets or environment variable injection
6. **Run as Non-Root**: Container runs as `mpi` user (UID 1000)
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
  mpi-server:
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

1. **Load Balancer**: Use nginx or HAProxy in front of multiple MPI instances
2. **Shared Database**: All instances connect to same PostgreSQL
3. **Shared Search Index**: Use network-mounted search index or separate search service
4. **Stateless Design**: MPI server is stateless, scales horizontally

Example:

```bash
docker-compose up -d --scale mpi-server=3
```

### Vertical Scaling

Increase resources for single instance:

```yaml
services:
  mpi-server:
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

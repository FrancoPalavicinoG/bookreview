# BookReview

## Quick Start - Docker Compose Options

This project provides multiple Docker Compose configurations for different deployment scenarios:

| File | Description | Services | Access URL | Use Case |
|------|-------------|----------|------------|----------|
| `docker-compose.basic.yml` | **Application + Database** | Web + MongoDB | `http://localhost:8000` | Development, Testing |
| `docker-compose.cache.yml` | **Application + Database + Redis** | Web + MongoDB + Redis | `http://localhost:8000` | Development, Testing |
| `docker-compose.proxy.yml` | **Application + Database + Reverse Proxy** | Apache + Web + MongoDB | `http://app.localhost` | Production (without caching) |
| `docker-compose.production.yml` | **Application + Database + Reverse Proxy + Redis** | Apache + Web + MongoDB + Redis | `http://app.localhost` | Full Production |
| `docker-compose.yml` | **Default (same as production)** | Apache + Web + MongoDB + Redis | `http://app.localhost` | Full Production |
| `docker-compose.dev.yml` | **Legacy Development** | Web + MongoDB | `http://localhost:8000` | Backward compatibility |

### Quick Commands

```bash
# Basic setup (recommended for development)
docker compose -f docker-compose.basic.yml up -d --build

# Full production setup with Redis caching
docker compose up -d --build  # Uses docker-compose.yml (production setup)

# Production setup without caching (legacy)
docker compose -f docker-compose.proxy.yml up -d --build
```

---

## 1) Instalar Rust (macOS con Homebrew)

```bash
# Instalar rustup (gestor oficial de toolchains de Rust) vía Homebrew
brew install rustup-init

# Inicializar e instalar toolchain estable
rustup-init -y

# Cargar el entorno en la sesión actual
source "$HOME/.cargo/env"

# Verificar que quedó OK
rustc --version
cargo --version
```

### Alternativas
- **macOS (Homebrew, sin rustup):**
  ```bash
  brew install rust   # instala rustc/cargo directamente (actualiza con brew upgrade)
  ```
- **Linux / WSL2 (oficial):**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```
- **Windows:** usar **Rustup** para Windows o WSL2 + alguno de los métodos anteriores.

---

## 2) Levantar MongoDB con Docker
1. Tener **Docker Desktop** (en macOS puedes instalarlo con Homebrew):
   ```bash
   brew install --cask docker
   ```
   Abre Docker Desktop al menos una vez.

---

## 3) Variables de entorno (`.env`)
Crea un archivo **`.env`** en la **raíz** del repo con:

```env
# Conexión a Mongo local (publicado por docker compose)
MONGO_URI=mongodb://localhost:27017

# Nombre de la base de datos (dev)
DB_NAME=bookreview_dev

# Static file serving (true/false)
SERVE_STATIC_FILES=true

# Uploads directory path
UPLOADS_DIR=uploads
```

**Tip:** Puedes usar el archivo `.env.example` como plantilla:
```bash
cp .env.example .env
```

---

## 4) Docker Compose Deployment Options

The project provides multiple Docker Compose configurations for different deployment scenarios:

### 4.1. Application + Database (Basic Setup)

**File:** `docker-compose.basic.yml`  
**Use case:** Development, testing, simple deployments  
**Services:** Web application + MongoDB  
**Static files:** Served directly by the Rust application  

```bash
# Start services
docker compose -f docker-compose.basic.yml up -d --build

# View logs
docker compose -f docker-compose.basic.yml logs -f web

# Stop services
docker compose -f docker-compose.basic.yml down
```

**Access:** `http://localhost:8000`

### 4.2. Application + Database + Reverse Proxy (Basic Production Setup)

**File:** `docker-compose.proxy.yml`  
**Use case:** Production deployments without caching, load balancing, SSL termination  
**Services:** Apache reverse proxy + Web application + MongoDB  
**Static files:** Served by Apache reverse proxy  

```bash
# Start services
docker compose -f docker-compose.proxy.yml up -d --build

# View logs
docker compose -f docker-compose.proxy.yml logs -f apache
docker compose -f docker-compose.proxy.yml logs -f web

# Stop services
docker compose -f docker-compose.proxy.yml down
```

**Important:** Add this line to your `/etc/hosts` file:
```
127.0.0.1 app.localhost
```

**Access:** `http://app.localhost`

### 4.3. Application + Database + Redis (Basic + Cache)

**Files:** `docker-compose.basic.yml` + `docker-compose.cache.yml`  
**Use case:** Development with Redis caching (no reverse proxy)  
**Services:** Web application + MongoDB + Redis  
**Access:** `http://localhost:8000`

```bash
# Start services (basic + cache)
docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml up -d --build

# View logs
docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml logs -f web
docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml logs -f redis

# Stop services
docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml down
```

- Enables Redis via `CACHE_URL` and builds the web with `CARGO_FEATURES=redis-cache`.

### 4.4. Application + Database + Reverse Proxy + Redis (Full Production Setup)

**File:** `docker-compose.production.yml` or `docker-compose.yml` (default)  
**Use case:** Full production deployments with Redis caching, reverse proxy, and database  
**Services:** Apache reverse proxy + Web application + MongoDB + Redis cache  
**Static files:** Served by Apache reverse proxy  
**Caching:** Redis for improved performance  

```bash
# Start services (using default compose file)
docker compose up -d --build

# Or explicitly use production file
docker compose -f docker-compose.production.yml up -d --build

# View logs
docker compose logs -f apache
docker compose logs -f web
docker compose logs -f redis

# Stop services
docker compose down
```

**Important:** Add this line to your `/etc/hosts` file:
```
127.0.0.1 app.localhost
```

**Access:** `http://app.localhost`

**Redis Features:**
- **Cache TTL**: Configurable time-to-live for cached data
- **Memory Management**: LRU eviction policy with 256MB limit
- **Persistence**: Redis persistence enabled for cache recovery
- **Health Checks**: Automatic Redis health monitoring

### 4.5. Legacy Development Mode

**File:** `docker-compose.dev.yml`  
**Note:** This file is maintained for backward compatibility. Use `docker-compose.basic.yml` for new projects.

```bash
docker compose -f docker-compose.dev.yml up -d --build
```

### 4.6. Seeder (Load Sample Data)

Load sample data into the database for any deployment:

```bash
# For basic setup
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# For production setup with Redis (default)
docker compose run --rm web sh -lc '/app/seeder'

# For proxy setup (without Redis)
docker compose -f docker-compose.proxy.yml run --rm web sh -lc '/app/seeder'

# For legacy dev setup
docker compose -f docker-compose.dev.yml run --rm web sh -lc '/app/seeder'

# For legacy + redis
docker compose exec web /app/seeder   
```

### 4.7. Testing Image Uploads

1. Go to `/upload` page in the application
2. Upload book covers and author images
3. Test static file access:
   - **Basic setup**: `http://localhost:8000/static/filename`
   - **Production/Proxy setup**: `http://app.localhost/static/filename`

---

## 5) Deployment Comparison

| Feature | Basic Setup | Proxy Setup | Production Setup |
|---------|-------------|-------------|------------------|
| **File** | `docker-compose.basic.yml` | `docker-compose.proxy.yml` | `docker-compose.production.yml` |
| **Services** | Web + MongoDB | Apache + Web + MongoDB | Apache + Web + MongoDB + Redis |
| **Static Files** | Served by Rust app | Served by Apache | Served by Apache |
| **Caching** | None | None | Redis |
| **URL** | `http://localhost:8000` | `http://app.localhost` | `http://app.localhost` |
| **Use Case** | Development, Testing | Production (basic) | Production (full-featured) |
| **SSL Support** | Manual setup required | Easy Apache config | Easy Apache config |
| **Performance** | Good for dev | Better for production | Best for production |
| **Complexity** | Simple | Medium | Medium-High |
| **Memory Usage** | Low | Medium | Medium-High |
| **Cache Features** | - | - | TTL, LRU eviction, persistence |

---

## 6) Architecture Overview

### Deployment Architectures

The application supports three main deployment architectures:

**Basic Architecture (Application + Database):**
```
[Client] → [Rust Web App:8000] → [MongoDB:27017]
                ↓
         [Static Files Served by App]
```

**Proxy Architecture (Application + Database + Reverse Proxy):**
```
[Client] → [Apache:80] → [Rust Web App:8000] → [MongoDB:27017]
              ↓
       [Static Files Served by Apache]
```

**Production Architecture (Application + Database + Reverse Proxy + Redis Cache):**
```
[Client] → [Apache:80] → [Rust Web App:8000] → [MongoDB:27017]
              ↓              ↓ ↑
       [Static Files]   [Redis Cache:6379]
       [Served by Apache]
```

### Configuration Differences

**Basic Setup (`docker-compose.basic.yml`):**
- Application serves static files directly
- `SERVE_STATIC_FILES=true`
- No caching layer
- Direct access via `http://localhost:8000`
- Simpler setup, ideal for development

**Proxy Setup (`docker-compose.proxy.yml`):**
- Apache serves static files
- Application focuses on dynamic content
- `SERVE_STATIC_FILES=false`
- No caching layer
- Access via `http://app.localhost`
- Better for basic production

**Production Setup (`docker-compose.production.yml`):**
- Apache serves static files
- Application focuses on dynamic content
- `SERVE_STATIC_FILES=false`
- Redis caching enabled via `CACHE_URL`
- Cargo features: `redis-cache`
- Access via `http://app.localhost`
- Best for high-performance production

### Image Upload System

- **Supported formats**: JPG, JPEG, PNG, GIF, WebP
- **Storage**: Configurable directory via `UPLOADS_DIR`
- **Organization**: Automatic categorization by type (book covers, author images)
- **Unique naming**: UUID-based filenames to prevent conflicts

### Environment Variables

| Variable | Description | Basic Setup | Proxy Setup | Production Setup |
|----------|-------------|-------------|-------------|------------------|
| `SERVE_STATIC_FILES` | Whether app serves static files | `true` | `false` | `false` |
| `CACHE_URL` | Redis connection string | Not set | Not set | `redis://redis:6379` |
| `UPLOADS_DIR` | Directory for uploaded files | `/app/uploads` | `/app/uploads` | `/app/uploads` |
| `MONGO_URI` | MongoDB connection string | `mongodb://mongo:27017` | `mongodb://mongo:27017` | `mongodb://mongo:27017` |
| `DB_NAME` | Database name | `bookreview_dev` | `bookreview_dev` | `bookreview_dev` |

### Redis Cache Configuration

The production setup includes Redis with the following configuration:
- **Image**: `redis:7-alpine`
- **Memory Limit**: 256MB with LRU eviction policy
- **Persistence**: Append-only file (AOF) enabled
- **Network**: Optimized with increased socket connections
- **Health Checks**: Automatic ping monitoring
- **TTL Support**: Configurable time-to-live for cached data

---

## 7) API Endpoints

### Core Routes
- `GET /` - Home page with dashboard
- `GET /health` - Health check endpoint
- `GET /upload` - Image upload interface
- `POST /upload` - Handle file uploads

### Resource Routes
- `/authors/*` - Author management
- `/books/*` - Book management  
- `/reviews/*` - Review management
- `/sales/*` - Sales data management

### Static Files
- `/static/*` - Static file serving (when not behind proxy)

---

## 8) Docker Commands Reference

### Build and Run Commands

```bash
# Basic setup (Application + Database)
docker compose -f docker-compose.basic.yml up -d --build

# Proxy setup (Application + Database + Reverse Proxy)
docker compose -f docker-compose.proxy.yml up -d --build

# Production setup (Application + Database + Reverse Proxy + Redis)
docker compose -f docker-compose.production.yml up -d --build
# or using default file
docker compose up -d --build

# Legacy development setup
docker compose -f docker-compose.dev.yml up -d --build
```

### View Logs

```bash
# Basic setup
docker compose -f docker-compose.basic.yml logs -f web
docker compose -f docker-compose.basic.yml logs -f mongo

# Proxy setup
docker compose -f docker-compose.proxy.yml logs -f apache
docker compose -f docker-compose.proxy.yml logs -f web
docker compose -f docker-compose.proxy.yml logs -f mongo

# Production setup (default)
docker compose logs -f apache
docker compose logs -f web
docker compose logs -f redis
docker compose logs -f mongo

# All services at once
docker compose logs -f
```

### Stop Services

```bash
# Basic setup
docker compose -f docker-compose.basic.yml down

# Proxy setup
docker compose -f docker-compose.proxy.yml down

# Production setup (default)
docker compose down

# Stop and remove volumes (reset database and cache)
docker compose down -v
```

### Reset and Rebuild

```bash
# Complete reset with fresh build (basic)
docker compose -f docker-compose.basic.yml down -v
docker compose -f docker-compose.basic.yml up -d --build

# Complete reset with fresh build (proxy)
docker compose -f docker-compose.proxy.yml down -v
docker compose -f docker-compose.proxy.yml up -d --build

# Complete reset with fresh build (production)
docker compose down -v
docker compose up -d --build
```

### Seeder Commands

```bash
# Load sample data - basic setup
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# Load sample data - proxy setup
docker compose -f docker-compose.proxy.yml run --rm web sh -lc '/app/seeder'

# Load sample data - production setup
docker compose run --rm web sh -lc '/app/seeder'
```

---

## 9) Testing Different Deployments

### Testing Basic Setup

```bash
# Start basic setup
docker compose -f docker-compose.basic.yml up -d --build

# Test application
curl http://localhost:8000/health

# Test static file serving (upload a file first via web interface)
curl -I http://localhost:8000/static/your-uploaded-file.jpg
# Should show Rocket/Rust headers

# Load sample data
docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder'

# Stop
docker compose -f docker-compose.basic.yml down
```

### Testing Proxy Setup

```bash
# Add to /etc/hosts first
echo "127.0.0.1 app.localhost" | sudo tee -a /etc/hosts

# Start proxy setup (without Redis)
docker compose -f docker-compose.proxy.yml up -d --build

# Test application through proxy
curl http://app.localhost/health

# Test static file serving through Apache
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers

# Load sample data
docker compose -f docker-compose.proxy.yml run --rm web sh -lc '/app/seeder'

# Stop
docker compose -f docker-compose.proxy.yml down
```

### Testing Production Setup

```bash
# Add to /etc/hosts first
echo "127.0.0.1 app.localhost" | sudo tee -a /etc/hosts

# Start production setup with Redis
docker compose up -d --build

# Test application through proxy
curl http://app.localhost/health

# Test static file serving through Apache
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers

# Test Redis cache (check logs for cache hits/misses)
docker compose logs web | grep cache

# Load sample data
docker compose run --rm web sh -lc '/app/seeder'

# Stop
docker compose down
```

### Comparing All Setups

```bash
# Start basic setup on port 8000
docker compose -f docker-compose.basic.yml up -d --build

# In another terminal, start proxy setup
docker compose -f docker-compose.proxy.yml up -d --build

# In another terminal, start production setup  
docker compose up -d --build

# Now you can compare:
# Basic: http://localhost:8000
# Proxy: http://app.localhost (via proxy without cache)
# Production: http://app.localhost (via proxy with Redis cache)

# Don't forget to stop all when done
docker compose -f docker-compose.basic.yml down
docker compose -f docker-compose.proxy.yml down
docker compose down
```

---

## 10) Testing the Reverse Proxy and Redis Cache

### 1. Setup hosts file
Add to `/etc/hosts`:
```
127.0.0.1 app.localhost
```

### 2. Start production setup
```bash
docker compose up -d --build
# or explicitly
docker compose -f docker-compose.production.yml up -d --build
```

### 3. Test static file serving
```bash
# Upload a test image via the web interface at:
http://app.localhost/upload

# Verify Apache serves static files by checking headers:
curl -I http://app.localhost/static/your-uploaded-file.jpg
# Should show Apache headers (Server: Apache/2.4.x)
```

### 4. Test Redis caching 

> Set the base URL:

```bash
# Proxy
export BASE_URL="http://app.localhost"
# Legacy
# export BASE_URL="http://localhost:8000"
```

#### 4.1 Check Redis is up
```bash
docker compose exec redis redis-cli ping
# Expected: PONG
```

```bash
docker compose logs --no-log-prefix web | grep "^\[cache\]"
# Expected: [cache] Using Redis at redis://redis:6379
```

#### 4.2 Author information (Authors summary cache) 
```bash
# First MISS -> Mongo DB, Second redis -> HIT
curl -s "$BASE_URL/" > /dev/null
curl -s "$BASE_URL/" > /dev/null

# Check the cached key TTL > 0
docker compose exec -T redis redis-cli TTL authors:summary
```

#### 4.3 Most common queries (Search cache)
```bash
# (MISS then HIT)
curl -s "$BASE_URL/search?q=the&page=1" > /dev/null
curl -s "$BASE_URL/search?q=the&page=1" > /dev/null

# TTL > 0
docker compose exec -T redis redis-cli TTL "search:books:q:the:p:1:pp:10"
```

#### 4.4 Reviews Scores (Book average score cache)
```bash
# Get DB name from the running app
DB_NAME=$(docker compose exec -T web sh -lc 'printf "%s" "${DB_NAME:-bookreview_dev}"' | tr -d '\r')

# Pick a book with the most reviews
BOOK_ID=$(
  docker compose exec -T mongo mongosh --quiet --eval "
    const db=db.getSiblingDB('$DB_NAME');
    const d=db.reviews.aggregate([
      { \$group:{ _id:'\$book_id', c:{ \$sum:1 } } },
      { \$sort:{ c:-1 } },
      { \$limit:1 }
    ]).toArray()[0];
    if (d) print(d._id.toHexString());
  " | tr -d '\r'
)
echo "BOOK_ID=$BOOK_ID"

# First MISS -> Mongo DB, Second redis -> HIT
curl -s "$BASE_URL/books/avg/${BOOK_ID}" && echo
curl -s "$BASE_URL/books/avg/${BOOK_ID}" && echo

# TTL > 0
docker compose exec -T redis redis-cli TTL "book:${BOOK_ID}:avg_score"
```

#### 4.5 CRUD purge test 
Creating a **new review** for that `BOOK_ID` should purge:
- `book:&lt;ID&gt;:avg_score` 
- `authors:summary` 
- the search cache 

```bash
# Prime keys 
curl -s "$BASE_URL/" > /dev/null
curl -s "$BASE_URL/search?q=the&page=1" > /dev/null
curl -s "$BASE_URL/books/avg/${BOOK_ID}" > /dev/null

# Create a review (HTTP 303 expected)
curl -s -i -X POST "$BASE_URL/reviews/create" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  --data-urlencode "book_id=${BOOK_ID}" \
  --data-urlencode "text=cache invalidate test" \
  --data-urlencode "score=5" \
  --data-urlencode "up_votes=0" | head -n1

# All related keys should be gone (TTL = -2)
docker compose exec -T redis redis-cli TTL "book:${BOOK_ID}:avg_score"
docker compose exec -T redis redis-cli TTL authors:summary
docker compose exec -T redis redis-cli TTL "search:books:q:the:p:1:pp:10"
```

> To **watch MISS/HIT** in real time, run:
> ```bash
> docker compose logs -f web | grep --line-buffered '\[cache\]'
> ```


### 5. Test application routing
```bash
# Health check through proxy
curl http://app.localhost/health

# API endpoints through proxy  
curl http://app.localhost/authors
curl http://app.localhost/books
```

### 6. Compare with basic setup
```bash
# Stop production setup
docker compose down

# Start basic setup
docker compose -f docker-compose.basic.yml up -d --build

# Test direct access (no caching)
curl http://localhost:8000/health
curl -I http://localhost:8000/static/your-file.jpg
# Should show Rocket headers (Server: Rocket)

# Performance comparison (no cache)
for i in {1..5}; do
  curl -w "Time: %{time_total}s\n" -o /dev/null -s http://localhost:8000/books
done
# All requests should take similar time (no caching)
```

---

## 11) Ejecutar la app en **Kubernetes**

### Prerrequisitos
- `kubectl` instalado.
- **Cluster local**:
  - **kind** (Kubernetes in Docker).


---

### Usando **kind** (recomendado)
1) **Crear** un cluster local:
```bash
kind create cluster --name bookreview
```

2) **Construir** la imagen de la web (local):
```bash
docker build -t bookreview-web:kind .
```

3) **Cargar** la imagen al cluster kind:
```bash
kind load docker-image bookreview-web:kind --name bookreview
```

4) **Aplicar** los manifiestos:
```bash
kubectl apply -f k8s/bookreview.yaml
```

5) **Ver pods** hasta que estén Ready:
```bash
kubectl -n bookreview get pods -w
```

6) **Exponer** localmente con port-forward y abrir la web:
```bash
kubectl -n bookreview port-forward svc/bookreview-web 8000:8000
# Luego visita: http://127.0.0.1:8000/
```

**Seeder** (carga de datos de ejemplo) en el cluster:
```bash
kubectl -n bookreview run seeder --image=bookreview-web:kind --restart=Never --rm -it --env="MONGO_URI=mongodb://mongo:27017" --env="DB_NAME=bookreview_dev" -- /app/seeder
```

---

### Comandos útiles
- **Logs** de la web:
```bash
kubectl -n bookreview logs deploy/bookreview-web -f
```

- **Reiniciar** la web (rolling restart):
```bash
kubectl -n bookreview rollout restart deploy bookreview-web
kubectl -n bookreview rollout status deploy/bookreview-web
```

- **Actualizar** la imagen tras recompilar (kind):
```bash
docker build -t bookreview-web:kind .
kind load docker-image bookreview-web:kind --name bookreview
kubectl -n bookreview rollout restart deploy bookreview-web
```

- **Eliminar** todo lo desplegado:
```bash
kubectl delete -f k8s/bookreview.yaml
```

- **Borrar** el cluster kind (si usaste kind):
```bash
kind delete cluster --name bookreview
```

---

## 13) Load Testing

The project includes comprehensive JMeter load tests to compare performance between deployment modes.

### Quick Load Testing

**Prerequisites:**
```bash
# Install JMeter (macOS)
brew install jmeter

# Verify installation
jmeter -v
```

**Run Simple Tests:**
```bash
cd load-testing

# Test basic deployment (app + database)
docker compose -f ../docker-compose.basic.yml up -d --build
./simple-test.sh basic 10  # 10 concurrent users

# Test proxy deployment (apache + app + database)  
docker compose -f ../docker-compose.yml up -d --build
./simple-test.sh proxy 10  # 10 concurrent users
```

**Collect System Metrics:**
```bash
# Monitor container resource usage during tests
./collect-metrics.sh basic 300   # Monitor for 5 minutes
./collect-metrics.sh proxy 300   # Monitor for 5 minutes
```

### Complete Test Suite

Run all test scenarios (1, 10, 100, 1000, 5000 users × 5 minutes each):

```bash
cd load-testing
./run-load-tests.sh  # Takes ~50 minutes total
```

### Test Results

Results are saved in `load-testing/results/`:
- **JMeter Results**: `.jtl` files with response times, throughput, error rates
- **HTML Reports**: Interactive dashboards in `*_report/` directories  
- **System Metrics**: CSV files with CPU, memory, network, disk usage
- **Summary Report**: `load_test_report.md` with comparison analysis

### Metrics Collected

**Application Performance:**
- Response times (min, max, average, percentiles)
- Throughput (requests per second)
- Error rates and HTTP status codes
- Request distribution across endpoints

**System Resources:**
- CPU usage percentage per container
- Memory usage (MB and percentage)
- Network I/O (received/transmitted data)
- Disk I/O (read/write operations)
- Process counts

### Analysis Framework

Compare the two deployment architectures across:

1. **Response Times**: Which deployment responds faster?
2. **Throughput**: Which can handle more requests per second?
3. **Resource Efficiency**: Which uses fewer system resources?
4. **Scalability**: Which handles high load better?
5. **Error Patterns**: Which deployment is more stable under stress?

For detailed instructions, see [`load-testing/README.md`](load-testing/README.md).

---

## 14) Troubleshooting

### Common Issues

**1. "app.localhost" not resolving**
- Ensure `/etc/hosts` contains: `127.0.0.1 app.localhost`
- Try clearing DNS cache: `sudo dscacheutil -flushcache` (macOS)

**2. Static files not loading behind proxy**
- Check Apache container logs: `docker compose logs apache`
- Verify uploads volume is mounted correctly
- Ensure file permissions are correct

**3. File upload fails**
- Check uploads directory exists and is writable
- Verify file size limits in Apache configuration
- Check application logs for detailed errors

**4. Database connection issues**
- Ensure MongoDB container is healthy: `docker compose ps`
- Check network connectivity between containers
- Verify environment variables are set correctly

**6. Redis connection issues**
- Check Redis container is healthy: `docker compose ps redis`
- Test Redis connectivity: `docker compose exec redis redis-cli ping`
- Check Redis logs: `docker compose logs redis`
- Verify `CACHE_URL` environment variable is set correctly

**7. Port conflicts**
- Basic setup uses port 8000: ensure it's not in use
- Production/Proxy setup uses port 80: ensure it's not in use
- Redis uses port 6379: ensure it's not in use
- Check running processes: `lsof -i :8000` or `lsof -i :80` or `lsof -i :6379`

### Debug Commands

```bash
# Check container status for different setups
docker compose -f docker-compose.basic.yml ps
docker compose -f docker-compose.proxy.yml ps
docker compose ps

# View all logs for basic setup
docker compose -f docker-compose.basic.yml logs

# View all logs for proxy setup
docker compose -f docker-compose.proxy.yml logs

# View all logs for production setup
docker compose logs

# Test Redis connectivity (production setup only)
docker compose exec redis redis-cli ping
docker compose exec redis redis-cli info stats

# Inspect uploads volume
docker volume inspect bookreview_uploads_data

# Inspect Redis data volume
docker volume inspect bookreview_redis_data

# Test file upload via curl (basic setup)
curl -X POST -F "file=@test.jpg" -F "upload_type=book_cover" -F "entity_id=test" http://localhost:8000/upload

# Test file upload via curl (proxy setup)
curl -X POST -F "file=@test.jpg" -F "upload_type=book_cover" -F "entity_id=test" http://app.localhost/upload

# Test static file serving
curl -I http://localhost:8000/static/test_book_cover_*.jpg  # Basic setup
curl -I http://app.localhost/static/test_book_cover_*.jpg   # Proxy setup
```

### Quick Setup Verification

```bash
# Verify basic setup is working
docker compose -f docker-compose.basic.yml up -d --build
curl http://localhost:8000/health
# Should return: {"status": "healthy"}

# Verify proxy setup is working
docker compose up -d --build
curl http://app.localhost/health
# Should return: {"status": "healthy"}
```

#!/bin/bash

# 🚀 Fuel Cost Dashboard Deployment Script
# Interactive deployment script that asks for all configuration parameters
# and deploys the HTML dashboard to your VPS with Nginx configuration
#
# Features:
# - Interactive configuration (Domain, Port, VPS IP, Service Name)
# - SSL/HTTPS support with Let's Encrypt
# - Complete validation and error handling  
# - Monitoring setup
# - Comprehensive logging

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration variables (will be set by user input)
DOMAIN=""
PORT=""
VPS_USER="$(whoami)"  # Use current user instead of root
VPS_IP=""
SERVICE_NAME=""
LOCAL_HTML_FILE="index.html"
REMOTE_DIR=""
NGINX_SITE_CONFIG=""
SSL_ENABLED="n"
SSL_EMAIL=""
USE_REVERSE_PROXY="n"
PROXY_TARGET_PORT=""

# Logging functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

header() {
    echo -e "${CYAN}${BOLD}$1${NC}"
}

# Function to ask for user input with validation
ask_input() {
    local prompt="$1"
    local default="$2"
    local validation_func="$3"
    local response
    
    while true; do
        if [ -n "$default" ]; then
            read -p "$(echo -e "${CYAN}$prompt${NC} ${YELLOW}[default: $default]${NC}: ")" response
            response=${response:-$default}
        else
            read -p "$(echo -e "${CYAN}$prompt${NC}: ")" response
        fi
        
        if [ -z "$response" ]; then
            error "This field cannot be empty. Please enter a value."
            continue
        fi
        
        if [ -z "$validation_func" ] || $validation_func "$response"; then
            echo "$response"
            break
        fi
    done
}

# Function to ask yes/no question
ask_yes_no() {
    local prompt="$1"
    local default="$2"
    local response
    
    while true; do
        if [ "$default" = "y" ]; then
            read -p "$(echo -e "${CYAN}$prompt${NC} ${YELLOW}[Y/n]${NC}: ")" response
            response=${response:-y}
        elif [ "$default" = "n" ]; then
            read -p "$(echo -e "${CYAN}$prompt${NC} ${YELLOW}[y/N]${NC}: ")" response
            response=${response:-n}
        else
            read -p "$(echo -e "${CYAN}$prompt${NC} ${YELLOW}[y/n]${NC}: ")" response
        fi
        
        case "$response" in
            [Yy]|[Yy][Ee][Ss]) echo "y"; break ;;
            [Nn]|[Nn][Oo]) echo "n"; break ;;
            *) echo -e "${RED}Please answer yes (y) or no (n)${NC}" ;;
        esac
    done
}

# Validation functions
validate_ip() {
    local ip="$1"
    if [[ $ip =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]]; then
        IFS='.' read -ra ADDR <<< "$ip"
        for i in "${ADDR[@]}"; do
            if (( i > 255 )); then
                error "Invalid IP address: $ip (octet $i is greater than 255)"
                return 1
            fi
        done
        return 0
    else
        error "Invalid IP address format: $ip (expected: xxx.xxx.xxx.xxx)"
        return 1
    fi
}

validate_port() {
    local port="$1"
    if [[ $port =~ ^[0-9]+$ ]] && [ "$port" -ge 1 ] && [ "$port" -le 65535 ]; then
        if [ "$port" -lt 1024 ]; then
            warning "Port $port is a system port (< 1024). Make sure you have proper permissions."
        fi
        return 0
    else
        error "Invalid port number: $port (must be between 1-65535)"
        return 1
    fi
}

validate_domain() {
    local domain="$1"
    # Allow domains with underscores, multiple subdomains, and various formats
    if [[ $domain =~ ^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)*\.[a-zA-Z]{2,}$ ]] || [[ $domain =~ ^[a-zA-Z0-9_.-]+$ ]]; then
        return 0
    else
        error "Invalid domain format: $domain (expected: example.com, subdomain.example.com, or sub_domain.example.com)"
        return 1
    fi
}

validate_service_name() {
    local name="$1"
    if [[ $name =~ ^[a-zA-Z0-9_-]+$ ]]; then
        return 0
    else
        error "Invalid service name: $name (only letters, numbers, hyphens and underscores allowed)"
        return 1
    fi
}

validate_email() {
    local email="$1"
    if [[ $email =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
        return 0
    else
        error "Invalid email format: $email"
        return 1
    fi
}

# Display welcome banner
clear
header "========================================================="
header "  🚀 Interactive Fuel Cost Dashboard Deployment Script  "
header "========================================================="
echo ""
info "This script will interactively configure and deploy your dashboard."
echo ""

# Interactive configuration
header "📋 Configuration Setup"
echo ""

# Ask for Domain
info "📌 Step 1: Domain Configuration"
DOMAIN=$(ask_input "Enter your dashboard domain (e.g., dashboard.example.com)" "fuelcost_dashboard.blackshadow.software" "validate_domain")
info "✅ Domain set to: $DOMAIN"
echo ""

# Ask for Port
info "📌 Step 2: Port Configuration"
info "💡 Common ports: 8080, 8090, 8880, 3000, 5000"
PORT=$(ask_input "Enter the port for your dashboard" "8890" "validate_port")
info "✅ Port set to: $PORT"
echo ""

# Ask for VPS IP
info "📌 Step 3: VPS Configuration"
VPS_IP=$(ask_input "Enter your VPS IP address" "" "validate_ip")
info "✅ VPS IP set to: $VPS_IP"

# Ask for VPS username
info "💡 The script will use SSH with sudo instead of direct root access"
VPS_USER=$(ask_input "Enter your VPS username (user with sudo privileges)" "$(whoami)")
info "✅ VPS user set to: $VPS_USER"
echo ""

# Ask for Service Name
info "📌 Step 4: Service Configuration"
info "💡 This will be used for service names and directory names"
SERVICE_NAME=$(ask_input "Enter service name (alphanumeric, hyphens, underscores only)" "fuelcost_dashboard" "validate_service_name")
info "✅ Service name set to: $SERVICE_NAME"
echo ""

# Set derived configuration
REMOTE_DIR="/var/www/$SERVICE_NAME"
NGINX_SITE_CONFIG="/etc/nginx/sites-available/$DOMAIN"

# Ask about reverse proxy setup
info "📌 Step 5: Reverse Proxy Configuration"
info "💡 If port 80 is used by another service, we can set up Nginx as a reverse proxy"
USE_REVERSE_PROXY=$(ask_yes_no "Set up Nginx as reverse proxy (recommended if port 80 is busy)?" "y")

if [ "$USE_REVERSE_PROXY" = "y" ]; then
    info "✅ Will configure Nginx to reverse proxy to your service on port $PORT"
    info "📍 Your dashboard will be accessible at: http://$DOMAIN (no port needed)"
    PROXY_TARGET_PORT="$PORT"
    PORT="80"  # Nginx will listen on 80 and proxy to the target port
else
    info "✅ Will use direct port configuration"
fi

# Ask for SSL preference
info "📌 Step 6: SSL/HTTPS Configuration"
SSL_ENABLED=$(ask_yes_no "Do you want to enable SSL/HTTPS with Let's Encrypt?" "n")

if [ "$SSL_ENABLED" = "y" ]; then
    info "📧 SSL certificate requires a valid email address for Let's Encrypt"
    SSL_EMAIL=$(ask_input "Enter your email for SSL certificate" "" "validate_email")
    info "✅ SSL will be enabled with email: $SSL_EMAIL"
    
    # For SSL, we typically use standard ports
    SSL_SETUP_WARNING=$(ask_yes_no "⚠️ SSL setup will use standard ports 80/443. Your custom port $PORT will be ignored. Continue?" "y")
    if [ "$SSL_SETUP_WARNING" = "n" ]; then
        warning "SSL setup cancelled. Proceeding with HTTP only on port $PORT."
        SSL_ENABLED="n"
    fi
else
    info "✅ Proceeding with HTTP only on port $PORT"
fi

echo ""

# Display final configuration
header "📋 Final Configuration Summary"
echo ""
info "Domain: $DOMAIN"
info "VPS IP: $VPS_IP"
info "VPS User: $VPS_USER (with sudo)"
if [ "$USE_REVERSE_PROXY" = "y" ]; then
    info "Setup: Reverse Proxy (Nginx:80 → Service:$PROXY_TARGET_PORT)"
    info "Access URL: http://$DOMAIN (no port needed)"
elif [ "$SSL_ENABLED" = "y" ]; then
    info "Ports: 80 (HTTP redirect), 443 (HTTPS)"
    info "SSL Email: $SSL_EMAIL"
else
    info "Port: $PORT"
fi
info "Service Name: $SERVICE_NAME"
info "Remote Directory: $REMOTE_DIR"
info "Nginx Config: $NGINX_SITE_CONFIG"
info "SSL Enabled: $([ "$SSL_ENABLED" = "y" ] && echo "Yes" || echo "No")"
info "Reverse Proxy: $([ "$USE_REVERSE_PROXY" = "y" ] && echo "Yes" || echo "No")"
echo ""

# Final confirmation
PROCEED=$(ask_yes_no "Proceed with deployment using the above configuration?" "y")
if [ "$PROCEED" = "n" ]; then
    warning "Deployment cancelled by user"
    exit 0
fi

echo ""
log "🚀 Starting deployment with your custom configuration..."

# Pre-deployment checks
log "🔍 Running pre-deployment checks..."

# Check if local HTML file exists
if [ ! -f "$LOCAL_HTML_FILE" ]; then
    error "$LOCAL_HTML_FILE not found in current directory"
    info "Please ensure index.html is in the same directory as this script"
    exit 1
fi

log "✅ Local HTML file found: $LOCAL_HTML_FILE"

# Check if SSH connection works
log "🔐 Testing SSH connection to VPS..."
if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $VPS_USER@$VPS_IP "echo 'SSH connection successful'" >/dev/null 2>&1; then
    error "Cannot connect to VPS. Please check:"
    error "  1. VPS IP address is correct: $VPS_IP"
    error "  2. SSH key authentication is set up"
    error "  3. VPS is running and accessible"
    error "  4. You have root access to the VPS"
    exit 1
fi

log "✅ SSH connection successful"

# Function to run commands on VPS with proper error handling
run_remote() {
    local cmd="$1"
    local description="${2:-Running remote command}"
    
    # Use sudo for commands that need root privileges
    if ! ssh -o StrictHostKeyChecking=no $VPS_USER@$VPS_IP "sudo $cmd"; then
        error "$description failed"
        return 1
    fi
    return 0
}

# Function to copy files to VPS with error handling
copy_to_vps() {
    local local_file="$1"
    local remote_path="$2"
    local description="${3:-Copying file}"
    
    if ! scp -o StrictHostKeyChecking=no "$local_file" $VPS_USER@$VPS_IP:"$remote_path"; then
        error "$description failed"
        return 1
    fi
    return 0
}

# Function to find next available port on VPS
find_available_port() {
    local start_port="$1"
    local max_attempts=50
    local current_port=$start_port
    
    log "🔍 Checking for available ports starting from $start_port..."
    
    for ((i=0; i<max_attempts; i++)); do
        if run_remote "! netstat -ln | grep -q ':$current_port '" "Checking port $current_port" 2>/dev/null; then
            log "✅ Found available port: $current_port"
            echo "$current_port"
            return 0
        else
            warning "Port $current_port is in use, trying next port..."
            ((current_port++))
        fi
    done
    
    error "Could not find available port after checking $max_attempts ports starting from $start_port"
    return 1
}

# Function to ask user for alternative port
ask_for_alternative_port() {
    local current_port="$1"
    local suggested_port=$((current_port + 1))
    
    warning "⚠️ Port $current_port is already in use!"
    
    # Show what's using the port
    info "📊 Checking what's using port $current_port..."
    run_remote "netstat -tlnp | grep :$current_port" "Showing port usage" || true
    
    echo ""
    warning "🚫 Port $current_port is not available. Please choose an alternative port."
    info "💡 Suggested alternatives: $suggested_port, $((suggested_port + 1)), $((suggested_port + 10))"
    echo ""
    
    # Ask user for alternative port
    local new_port
    new_port=$(ask_input "Enter an alternative port number" "$suggested_port" "validate_port")
    
    # Check if the new port is available
    if run_remote "netstat -ln | grep -q ':$new_port '" "Checking new port availability" 2>/dev/null; then
        warning "Port $new_port is also in use. Let's try another one."
        # Recursively ask until we find a free port
        ask_for_alternative_port "$new_port"
        return $?
    else
        log "✅ Port $new_port is available!"
        PORT="$new_port"
        return 0
    fi
}

# Function to handle Nginx startup with user-prompted port selection
start_nginx_with_port_handling() {
    local max_attempts=3
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        log "🚀 Attempt $attempt: Starting Nginx..."
        
        # Try to start Nginx
        if run_remote "systemctl start nginx" "Starting Nginx (attempt $attempt)" 2>/dev/null; then
            log "✅ Nginx started successfully"
            return 0
        fi
        
        # Check if it's a port conflict
        if run_remote "systemctl status nginx 2>&1 | grep -q 'Address already in use'" "Checking for port conflict"; then
            warning "🚫 Port conflict detected!"
            
            # Show what's using common ports
            log "📊 Current port usage:"
            run_remote "netstat -tlnp | grep ':80\\|:443\\|:8080\\|:8090\\|:$PORT'" "Listing used ports" || true
            
            # Ask user what they want to do
            echo ""
            warning "⚠️ There are port conflicts preventing Nginx from starting."
            info "Options:"
            info "  1. Stop conflicting services automatically (like Apache)"
            info "  2. Choose a different port for your service"
            echo ""
            
            local choice
            choice=$(ask_input "Choose option (1 for auto-fix, 2 for different port)" "2")
            
            if [ "$choice" = "1" ]; then
                # Try automatic conflict resolution
                warning "🔧 Attempting automatic conflict resolution..."
                
                # Stop Apache if it's running
                if run_remote "systemctl is-active apache2" "Checking Apache status" 2>/dev/null; then
                    warning "🛑 Stopping Apache which is using port 80..."
                    run_remote "systemctl stop apache2" "Stopping Apache" || true
                fi
                
                # Remove conflicting nginx sites
                run_remote "rm -f /etc/nginx/sites-enabled/default" "Removing default site" || true
                run_remote "find /etc/nginx/sites-enabled/ -name '*' -type l -exec rm -f {} +" "Removing conflicting sites" || true
                
            else
                # Ask for alternative port
                if [ "$SSL_ENABLED" = "n" ]; then
                    ask_for_alternative_port "$PORT"
                    
                    # Update nginx configuration with new port
                    log "📝 Updating Nginx configuration with new port $PORT..."
                    recreate_nginx_config
                else
                    error "SSL is enabled and requires ports 80/443. Please stop conflicting services manually."
                    return 1
                fi
            fi
        else
            # Different error, show details
            error "❌ Nginx failed to start for a different reason:"
            run_remote "systemctl status nginx" "Showing Nginx status" || true
            run_remote "journalctl -u nginx --no-pager -n 10" "Showing Nginx logs" || true
        fi
        
        ((attempt++))
        if [ $attempt -le $max_attempts ]; then
            log "⏳ Waiting 3 seconds before retry..."
            sleep 3
        fi
    done
    
    error "Failed to start Nginx after $max_attempts attempts"
    return 1
}

# Function to recreate nginx configuration (used when port changes)
recreate_nginx_config() {
    log "🔄 Recreating Nginx configuration with updated settings..."
    
    # Create new nginx config with updated port
    if [ "$SSL_ENABLED" = "n" ]; then
        # HTTP Only Configuration with updated port
        cat > /tmp/nginx_config_updated << EOF
# HTTP server for $SERVICE_NAME on port $PORT
server {
    listen $PORT;
    server_name $DOMAIN;
    
    # Document root for $SERVICE_NAME
    root $REMOTE_DIR;
    index index.html;
    
    # Main location block for serving static files
    location / {
        try_files \$uri \$uri/ =404;
        
        # Add CORS headers for API requests
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
    }
    
    # Handle OPTIONS requests for CORS preflight
    location ~* ^.+\.(OPTIONS)$ {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE';
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
        add_header 'Content-Type' 'text/plain; charset=utf-8';
        add_header 'Content-Length' 0;
        return 204;
    }
    
    # Handle static assets with caching
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
        
        # CORS headers for static assets too
        add_header 'Access-Control-Allow-Origin' '*';
    }
    
    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src 'self' 'unsafe-inline' 'unsafe-eval' https: http: data:; img-src 'self' data: https: http:;" always;
    
    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/x-javascript
        application/xml+rss
        application/javascript
        application/json
        application/xml
        text/html;
    
    # Access and error logs for $SERVICE_NAME
    access_log /var/log/nginx/${SERVICE_NAME}_access.log;
    error_log /var/log/nginx/${SERVICE_NAME}_error.log;
}
EOF
        
        # Upload updated config
        copy_to_vps "/tmp/nginx_config_updated" "$NGINX_SITE_CONFIG" "Updated Nginx configuration upload"
        rm -f /tmp/nginx_config_updated
        
        # Update firewall for new port
        log "🔥 Updating firewall for new port $PORT..."
        run_remote "ufw allow $PORT/tcp" "Allowing new port $PORT" || true
        
        log "✅ Nginx configuration updated for port $PORT"
    fi
}

# Check if Nginx is installed on VPS
log "🌐 Checking if Nginx is installed on VPS..."
if ! run_remote "which nginx" "Nginx installation check"; then
    error "Nginx is not installed on VPS."
    INSTALL_NGINX=$(ask_yes_no "Would you like to install Nginx automatically?" "y")
    
    if [ "$INSTALL_NGINX" = "y" ]; then
        log "📦 Installing Nginx..."
        if ! run_remote "apt update && apt install nginx -y" "Installing Nginx"; then
            error "Failed to install Nginx"
            exit 1
        fi
        log "✅ Nginx installed successfully"
    else
        error "Please install Nginx manually and run the script again"
        exit 1
    fi
fi

log "✅ Nginx is installed on VPS"

# Check if Nginx is running and start with smart port handling
log "🔍 Checking if Nginx is running..."
if ! run_remote "systemctl is-active --quiet nginx" "Nginx status check"; then
    warning "Nginx is not running. Starting Nginx with smart port conflict resolution..."
    
    # Enable Nginx to start on boot
    run_remote "systemctl enable nginx" "Enabling Nginx service" || true
    
    # Use smart port handling function
    if ! start_nginx_with_port_handling; then
        error "Failed to start Nginx even after conflict resolution attempts"
        warning "Manual intervention may be required. Please check:"
        warning "  1. What services are using ports: netstat -tlnp | grep ':80\\|:443'"
        warning "  2. Nginx error logs: journalctl -u nginx"
        warning "  3. System resources: df -h && free -h"
        exit 1
    fi
else
    log "✅ Nginx is already running"
fi

log "✅ Nginx is running and ready"

echo ""
log "📦 Step 1: Preparing VPS environment for service '$SERVICE_NAME'..."

# Create directory structure on VPS with proper permissions
if ! run_remote "mkdir -p $REMOTE_DIR" "Creating remote directory: $REMOTE_DIR"; then
    error "Failed to create remote directory"
    exit 1
fi

if ! run_remote "mkdir -p /var/log/nginx" "Creating Nginx log directory"; then
    error "Failed to create Nginx log directory"
    exit 1
fi

# Set proper ownership and permissions
if ! run_remote "chown -R www-data:www-data $REMOTE_DIR" "Setting directory ownership"; then
    error "Failed to set directory ownership"
    exit 1
fi

if ! run_remote "chmod -R 755 $REMOTE_DIR" "Setting directory permissions"; then
    error "Failed to set directory permissions"
    exit 1
fi

log "✅ VPS directories created and configured for '$SERVICE_NAME'"

echo ""
log "📁 Step 2: Uploading HTML file..."

# Copy HTML file to VPS
if ! copy_to_vps "$LOCAL_HTML_FILE" "$REMOTE_DIR/" "HTML file upload"; then
    error "Failed to upload HTML file"
    exit 1
fi

# Set proper permissions for the HTML file
if ! run_remote "chown www-data:www-data $REMOTE_DIR/index.html" "Setting HTML file ownership"; then
    error "Failed to set HTML file ownership"
    exit 1
fi

if ! run_remote "chmod 644 $REMOTE_DIR/index.html" "Setting HTML file permissions"; then
    error "Failed to set HTML file permissions"
    exit 1
fi

log "✅ HTML file uploaded to $REMOTE_DIR/"

# If using reverse proxy, set up a simple HTTP server to serve the static files
if [ "$USE_REVERSE_PROXY" = "y" ]; then
    echo ""
    log "🔧 Step 2b: Setting up static file server for reverse proxy..."
    
    # Create a simple Python HTTP server service
    cat > /tmp/simple_server.py << EOF
#!/usr/bin/env python3
import http.server
import socketserver
import os
import sys

PORT = $PROXY_TARGET_PORT
DIRECTORY = "$REMOTE_DIR"

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIRECTORY, **kwargs)
    
    def end_headers(self):
        # Add CORS headers
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS, PUT, DELETE')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        super().end_headers()

if __name__ == "__main__":
    os.chdir(DIRECTORY)
    with socketserver.TCPServer(("127.0.0.1", PORT), Handler) as httpd:
        print(f"Serving {DIRECTORY} at http://127.0.0.1:{PORT}")
        httpd.serve_forever()
EOF
    
    # Upload the server script
    if ! copy_to_vps "/tmp/simple_server.py" "/usr/local/bin/${SERVICE_NAME}_server.py" "Static file server upload"; then
        error "Failed to upload static file server"
        exit 1
    fi
    rm -f /tmp/simple_server.py
    
    # Make it executable
    run_remote "chmod +x /usr/local/bin/${SERVICE_NAME}_server.py" "Making server executable" || true
    
    # Create systemd service for the static server
    cat > /tmp/static_server.service << EOF
[Unit]
Description=$SERVICE_NAME Static File Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=$REMOTE_DIR
ExecStart=/usr/bin/python3 /usr/local/bin/${SERVICE_NAME}_server.py
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    
    # Upload and configure the service
    if ! copy_to_vps "/tmp/static_server.service" "/etc/systemd/system/${SERVICE_NAME}.service" "Service file upload"; then
        error "Failed to upload service file"
        exit 1
    fi
    rm -f /tmp/static_server.service
    
    # Enable and start the service
    run_remote "systemctl daemon-reload" "Reloading systemd"
    run_remote "systemctl enable ${SERVICE_NAME}.service" "Enabling static server service"
    run_remote "systemctl start ${SERVICE_NAME}.service" "Starting static server service"
    
    # Check if service is running
    if run_remote "systemctl is-active --quiet ${SERVICE_NAME}.service" "Checking service status"; then
        log "✅ Static file server started successfully on port $PROXY_TARGET_PORT"
    else
        warning "⚠️ Static file server may have issues. Checking status..."
        run_remote "systemctl status ${SERVICE_NAME}.service" "Showing service status" || true
    fi
fi

echo ""
log "🌐 Step 3: Creating Nginx configuration for '$DOMAIN'..."

# Create Nginx site configuration based on setup type
if [ "$USE_REVERSE_PROXY" = "y" ]; then
    # Reverse Proxy Configuration (Nginx listens on 80, proxies to service port)
    cat > /tmp/nginx_config << EOF
# Reverse Proxy server for $SERVICE_NAME
server {
    listen 80;
    server_name $DOMAIN;
    
    # Reverse proxy to the service
    location / {
        proxy_pass http://127.0.0.1:$PROXY_TARGET_PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_cache_bypass \$http_upgrade;
        proxy_read_timeout 86400;
        
        # Add CORS headers for API requests
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
    }
    
    # Handle OPTIONS requests for CORS preflight
    location = /options-handler {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE';
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
        add_header 'Content-Type' 'text/plain; charset=utf-8';
        add_header 'Content-Length' 0;
        return 204;
    }
    
    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src 'self' 'unsafe-inline' 'unsafe-eval' https: http: data:; img-src 'self' data: https: http:;" always;
    
    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/x-javascript
        application/xml+rss
        application/javascript
        application/json
        application/xml
        text/html;
    
    # Access and error logs for $SERVICE_NAME
    access_log /var/log/nginx/${SERVICE_NAME}_access.log;
    error_log /var/log/nginx/${SERVICE_NAME}_error.log;
}
EOF
elif [ "$SSL_ENABLED" = "y" ]; then
    # SSL Configuration (will redirect HTTP to HTTPS)
    cat > /tmp/nginx_config << EOF
# HTTP server (redirects to HTTPS)
server {
    listen 80;
    server_name $DOMAIN;
    
    # Redirect all HTTP requests to HTTPS
    return 301 https://\$server_name\$request_uri;
}

# HTTPS server for $SERVICE_NAME
server {
    listen 443 ssl http2;
    server_name $DOMAIN;
    
    # SSL certificate paths (will be configured by Certbot)
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;
    
    # SSL configuration
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:50m;
    ssl_stapling on;
    ssl_stapling_verify on;
    
    # Modern SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-SHA384;
    ssl_prefer_server_ciphers off;
    
    # Document root for $SERVICE_NAME
    root $REMOTE_DIR;
    index index.html;
    
    # Main location block for serving static files
    location / {
        try_files \$uri \$uri/ =404;
        
        # Add CORS headers for API requests
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
    }
    
    # Handle OPTIONS requests for CORS preflight
    location ~* ^.+\.(OPTIONS)$ {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE';
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
        add_header 'Content-Type' 'text/plain; charset=utf-8';
        add_header 'Content-Length' 0;
        return 204;
    }
    
    # Handle static assets with caching
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
        
        # CORS headers for static assets too
        add_header 'Access-Control-Allow-Origin' '*';
    }
    
    # Enhanced security headers for HTTPS
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src 'self' 'unsafe-inline' 'unsafe-eval' https: http: data:; img-src 'self' data: https: http:;" always;
    
    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/x-javascript
        application/xml+rss
        application/javascript
        application/json
        application/xml
        text/html;
    
    # Access and error logs for $SERVICE_NAME
    access_log /var/log/nginx/${SERVICE_NAME}_access.log;
    error_log /var/log/nginx/${SERVICE_NAME}_error.log;
}
EOF
else
    # HTTP Only Configuration using custom port
    cat > /tmp/nginx_config << EOF
# HTTP server for $SERVICE_NAME on custom port $PORT
server {
    listen $PORT;
    server_name $DOMAIN;
    
    # Document root for $SERVICE_NAME
    root $REMOTE_DIR;
    index index.html;
    
    # Main location block for serving static files
    location / {
        try_files \$uri \$uri/ =404;
        
        # Add CORS headers for API requests
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;
    }
    
    # Handle OPTIONS requests for CORS preflight
    location ~* ^.+\.(OPTIONS)$ {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, PUT, DELETE';
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization';
        add_header 'Content-Type' 'text/plain; charset=utf-8';
        add_header 'Content-Length' 0;
        return 204;
    }
    
    # Handle static assets with caching
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files \$uri =404;
        
        # CORS headers for static assets too
        add_header 'Access-Control-Allow-Origin' '*';
    }
    
    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src 'self' 'unsafe-inline' 'unsafe-eval' https: http: data:; img-src 'self' data: https: http:;" always;
    
    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/x-javascript
        application/xml+rss
        application/javascript
        application/json
        application/xml
        text/html;
    
    # Access and error logs for $SERVICE_NAME
    access_log /var/log/nginx/${SERVICE_NAME}_access.log;
    error_log /var/log/nginx/${SERVICE_NAME}_error.log;
}
EOF
fi

# Copy Nginx config to VPS
if ! copy_to_vps "/tmp/nginx_config" "$NGINX_SITE_CONFIG" "Nginx configuration upload"; then
    error "Failed to upload Nginx configuration"
    rm -f /tmp/nginx_config
    exit 1
fi

rm -f /tmp/nginx_config
log "✅ Nginx configuration created for '$DOMAIN'"

echo ""
log "⚙️ Step 4: Configuring Nginx..."

# Clean up any conflicting sites first
log "🧹 Cleaning up conflicting Nginx sites..."
run_remote "rm -f /etc/nginx/sites-enabled/default" "Removing default site" || true
run_remote "rm -f /etc/nginx/sites-enabled/fuelcost.blackshadow.software" "Removing conflicting fuelcost site" || true
run_remote "find /etc/nginx/sites-enabled/ -name '*blackshadow*' -type l -delete" "Removing other blackshadow sites" || true

# Enable the site and configure Nginx
if ! run_remote "ln -sf $NGINX_SITE_CONFIG /etc/nginx/sites-enabled/" "Enabling Nginx site"; then
    error "Failed to enable Nginx site"
    exit 1
fi

# Test Nginx configuration
log "🔍 Testing Nginx configuration..."
if run_remote "nginx -t" "Nginx configuration test"; then
    log "✅ Nginx configuration is valid"
else
    error "Nginx configuration test failed"
    error "Please check the configuration file: $NGINX_SITE_CONFIG"
    # Show the last few lines of nginx error log for debugging
    run_remote "tail -n 10 /var/log/nginx/error.log" "Showing Nginx error log" || true
    exit 1
fi

# Reload Nginx
if ! run_remote "systemctl reload nginx" "Reloading Nginx"; then
    error "Failed to reload Nginx"
    exit 1
fi

log "✅ Nginx configured and reloaded successfully"

echo ""
log "🔥 Step 5: Configuring firewall..."

# Check if UFW is installed and active
if run_remote "which ufw" "Checking UFW installation" 2>/dev/null; then
    if [ "$SSL_ENABLED" = "y" ]; then
        # For SSL, we need ports 80 and 443
        if run_remote "ufw allow 80/tcp && ufw allow 443/tcp" "Configuring firewall for SSL"; then
            log "✅ Firewall configured to allow ports 80 and 443 for SSL"
        else
            warning "Failed to configure firewall - you may need to manually open ports 80 and 443"
        fi
    else
        # For HTTP only, use the custom port
        if run_remote "ufw allow $PORT/tcp" "Configuring firewall for port $PORT"; then
            log "✅ Firewall configured to allow port $PORT"
        else
            warning "Failed to configure firewall - you may need to manually open port $PORT"
            info "💡 You can manually allow the port with: ufw allow $PORT/tcp"
        fi
    fi
else
    warning "UFW not found - please manually ensure required ports are open"
fi

echo ""

# SSL Certificate setup
if [ "$SSL_ENABLED" = "y" ]; then
    log "🔐 Step 6: Setting up SSL certificate with Let's Encrypt..."
    
    # Install Certbot
    if ! run_remote "which certbot" "Checking Certbot installation" 2>/dev/null; then
        log "📦 Installing Certbot..."
        if ! run_remote "apt update && apt install certbot python3-certbot-nginx -y" "Installing Certbot"; then
            error "Failed to install Certbot"
            exit 1
        fi
        log "✅ Certbot installed successfully"
    fi
    
    # Generate SSL certificate
    log "🎫 Generating SSL certificate for '$DOMAIN'..."
    if run_remote "certbot --nginx -d $DOMAIN --non-interactive --agree-tos --email $SSL_EMAIL --redirect" "Generating SSL certificate"; then
        log "✅ SSL certificate generated successfully for '$DOMAIN'"
        
        # Set up auto-renewal
        if run_remote "systemctl enable certbot.timer" "Setting up SSL auto-renewal"; then
            log "✅ SSL auto-renewal configured"
        else
            warning "Failed to set up SSL auto-renewal"
        fi
        
        TEST_URL="https://$DOMAIN"
        TEST_PORT=""
    else
        error "Failed to generate SSL certificate for '$DOMAIN'"
        warning "Falling back to HTTP configuration"
        SSL_ENABLED="n"
        TEST_URL="http://$DOMAIN"
        TEST_PORT=":$PORT"
    fi
elif [ "$USE_REVERSE_PROXY" = "y" ]; then
    TEST_URL="http://$DOMAIN"
    TEST_PORT=""
else
    TEST_URL="http://$DOMAIN"
    TEST_PORT=":$PORT"
fi

echo ""
log "🧪 Step $([ "$SSL_ENABLED" = "y" ] && echo "7" || echo "6"): Testing deployment..."

# Update user about any port changes
if [ "$SSL_ENABLED" = "n" ]; then
    info "🔗 Your dashboard will be accessible at: $TEST_URL$TEST_PORT"
fi

# Wait a moment for Nginx to fully reload
sleep 3

# Test if the site is accessible
log "Testing HTTP connection..."
if [ "$USE_REVERSE_PROXY" = "y" ]; then
    HTTP_STATUS=$(run_remote "curl -s -o /dev/null -w '%{http_code}' --connect-timeout 10 --max-time 30 http://localhost/ 2>/dev/null || echo 'FAILED'")
    # Also test the backend service
    BACKEND_STATUS=$(run_remote "curl -s -o /dev/null -w '%{http_code}' --connect-timeout 10 --max-time 30 http://localhost:$PROXY_TARGET_PORT/ 2>/dev/null || echo 'FAILED'")
    info "📊 Backend service (port $PROXY_TARGET_PORT): $BACKEND_STATUS"
elif [ "$SSL_ENABLED" = "y" ]; then
    HTTP_STATUS=$(run_remote "curl -s -o /dev/null -w '%{http_code}' --connect-timeout 10 --max-time 30 https://localhost/ 2>/dev/null || echo 'FAILED'")
else
    HTTP_STATUS=$(run_remote "curl -s -o /dev/null -w '%{http_code}' --connect-timeout 10 --max-time 30 http://localhost:$PORT/ 2>/dev/null || echo 'FAILED'")
fi

case "$HTTP_STATUS" in
    "200")
        log "✅ Local HTTP test successful (Status: $HTTP_STATUS)"
        ;;
    "FAILED"|"000"|"")
        error "❌ Local HTTP test failed - connection error"
        error "Please check:"
        error "  1. Nginx is running: systemctl status nginx"
        error "  2. Required ports are not blocked by firewall"
        error "  3. No other service is using the required ports"
        ;;
    *)
        warning "⚠️ Local HTTP test returned status: $HTTP_STATUS"
        if [ "$HTTP_STATUS" = "404" ]; then
            error "404 error - check if index.html exists in $REMOTE_DIR"
        fi
        ;;
esac

# Test if we can access the HTML content
log "Testing HTML content..."
if [ "$SSL_ENABLED" = "y" ]; then
    TEST_CMD="curl -s https://localhost/ | head -n 5 | grep -q 'html\\|HTML'"
else
    TEST_CMD="curl -s http://localhost:$PORT/ | head -n 5 | grep -q 'html\\|HTML'"
fi

if run_remote "$TEST_CMD" "HTML content test"; then
    log "✅ HTML content is accessible"
else
    warning "⚠️ Could not verify HTML content"
fi

echo ""
log "📊 Step $([ "$SSL_ENABLED" = "y" ] && echo "8" || echo "7"): Setting up monitoring for '$SERVICE_NAME'..."

# Create monitoring script
cat > /tmp/monitor_dashboard.sh << EOF
#!/bin/bash
# Enhanced monitoring script for $SERVICE_NAME dashboard

DOMAIN="$DOMAIN"
PORT="$PORT"
SSL_ENABLED="$SSL_ENABLED"
SERVICE_NAME="$SERVICE_NAME"
LOG_FILE="/var/log/${SERVICE_NAME}_monitor.log"

# Create log file if it doesn't exist
touch "\$LOG_FILE"

check_service() {
    local timestamp=\$(date '+%Y-%m-%d %H:%M:%S')
    local test_url
    
    if [ "\$SSL_ENABLED" = "y" ]; then
        test_url="https://localhost/"
    else
        test_url="http://localhost:\$PORT/"
    fi
    
    # Test HTTP response
    if curl -s -f --connect-timeout 10 --max-time 30 "\$test_url" > /dev/null 2>&1; then
        echo "[\$timestamp] \$SERVICE_NAME Dashboard is running - OK" >> "\$LOG_FILE"
        return 0
    else
        echo "[\$timestamp] \$SERVICE_NAME Dashboard is DOWN - ERROR" >> "\$LOG_FILE"
        
        # Additional diagnostics
        if ! systemctl is-active --quiet nginx; then
            echo "[\$timestamp] Nginx is not running" >> "\$LOG_FILE"
        fi
        
        if [ "\$SSL_ENABLED" = "y" ]; then
            if ! netstat -ln | grep -q ":443 "; then
                echo "[\$timestamp] Port 443 (HTTPS) is not listening" >> "\$LOG_FILE"
            fi
        else
            if ! netstat -ln | grep -q ":\$PORT "; then
                echo "[\$timestamp] Port \$PORT is not listening" >> "\$LOG_FILE"
            fi
        fi
        
        return 1
    fi
}

# Run check
check_service

# Keep only last 1000 lines of log
tail -n 1000 "\$LOG_FILE" > "\${LOG_FILE}.tmp" && mv "\${LOG_FILE}.tmp" "\$LOG_FILE"
EOF

if copy_to_vps "/tmp/monitor_dashboard.sh" "/usr/local/bin/monitor_${SERVICE_NAME}.sh" "Monitoring script upload"; then
    if run_remote "chmod +x /usr/local/bin/monitor_${SERVICE_NAME}.sh" "Setting monitoring script permissions"; then
        log "✅ Monitoring script installed as 'monitor_${SERVICE_NAME}.sh'"
        
        # Run initial monitoring check
        run_remote "/usr/local/bin/monitor_${SERVICE_NAME}.sh" "Running initial monitoring check" || true
    else
        warning "Failed to set monitoring script permissions"
    fi
else
    warning "Failed to upload monitoring script"
fi

rm -f /tmp/monitor_dashboard.sh

echo ""
header "🎉 DEPLOYMENT COMPLETED SUCCESSFULLY!"
echo ""
info "📋 Deployment Summary:"
info "  • Domain: $DOMAIN"
info "  • VPS IP: $VPS_IP"
info "  • Service Name: $SERVICE_NAME"
info "  • SSL Enabled: $([ "$SSL_ENABLED" = "y" ] && echo "Yes ($SSL_EMAIL)" || echo "No")"
if [ "$SSL_ENABLED" = "n" ]; then
    info "  • Port: $PORT"
fi
info "  • HTML File: $REMOTE_DIR/index.html"
info "  • Nginx Config: $NGINX_SITE_CONFIG"
info "  • HTTP Status: $HTTP_STATUS"
echo ""
header "🔗 Access your dashboard at:"
echo "   $TEST_URL$TEST_PORT"
echo ""
info "📝 Useful Commands:"
info "  • Check Nginx status: ssh $VPS_USER@$VPS_IP 'systemctl status nginx'"
info "  • View access logs: ssh $VPS_USER@$VPS_IP 'tail -f /var/log/nginx/${SERVICE_NAME}_access.log'"
info "  • View error logs: ssh $VPS_USER@$VPS_IP 'tail -f /var/log/nginx/${SERVICE_NAME}_error.log'"
if [ "$SSL_ENABLED" = "y" ]; then
    info "  • Check SSL certificate: ssh $VPS_USER@$VPS_IP 'certbot certificates'"
    info "  • Test SSL renewal: ssh $VPS_USER@$VPS_IP 'certbot renew --dry-run'"
fi
info "  • Monitor dashboard: ssh $VPS_USER@$VPS_IP '/usr/local/bin/monitor_${SERVICE_NAME}.sh'"
echo ""
info "🔧 To update the dashboard:"
info "  1. Modify index.html locally"
info "  2. Run this script again with the same configuration"
echo ""
warning "⚠️ Important Next Steps:"
warning "  1. Set up DNS A record for $DOMAIN pointing to $VPS_IP"
warning "  2. Test external access: $TEST_URL$TEST_PORT"
if [ "$SSL_ENABLED" = "n" ]; then
    warning "  3. Consider enabling SSL later by running this script again"
fi
warning "  4. Set up automated monitoring (cron job for monitor script)"
echo ""

if [ "$HTTP_STATUS" = "200" ]; then
    header "🎊 Deployment successful! Your '$SERVICE_NAME' dashboard should be accessible."
else
    warning "⚠️ Deployment completed but there may be issues. Check the logs and test manually."
fi

echo ""
info "Configuration used:"
info "  Domain: $DOMAIN | Port: $PORT | Service: $SERVICE_NAME | VPS: $VPS_IP"
info "Thank you for using the interactive dashboard deployment script! 🚀"
FROM nginx:alpine
# Replace default nginx.conf with our custom config (runs as root for Docker socket access)
COPY infra/dashboard/nginx.conf /etc/nginx/nginx.conf
COPY infra/dashboard/index.html /usr/share/nginx/html/index.html
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]

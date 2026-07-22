FROM nginx:alpine
COPY infra/dashboard/nginx.conf /etc/nginx/conf.d/default.conf
COPY infra/dashboard/index.html /usr/share/nginx/html/index.html
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]

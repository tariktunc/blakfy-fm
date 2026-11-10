FROM alpine:3
RUN apk add --no-cache ffmpeg python3
WORKDIR /app
COPY server.py .
EXPOSE 8000
CMD ["python3", "-u", "server.py"]

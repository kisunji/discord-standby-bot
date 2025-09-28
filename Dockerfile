# syntax=docker/dockerfile:1

# Build stage
FROM golang:1.24-alpine AS builder

# Install git and ca-certificates (git may be needed for private modules)
RUN apk add --no-cache git ca-certificates tzdata

# Set the working directory
WORKDIR /build

# Copy go mod and sum files
COPY go.mod go.sum ./

# Download dependencies
RUN go mod download && go mod verify

# Copy the source code
COPY . .

# Build the application
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build \
    -ldflags='-w -s -extldflags "-static"' \
    -a -installsuffix cgo \
    -o app .

# Final stage - distroless for security and minimal size
FROM gcr.io/distroless/static:nonroot

# Import from builder
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo

# Copy our static executable
COPY --from=builder /build/app /app

# Use nonroot user for security
USER nonroot:nonroot

# Expose port for metrics endpoint
EXPOSE 8080 2112

# Run the binary
ENTRYPOINT ["/app"]
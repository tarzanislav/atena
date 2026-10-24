FROM python:3.12-slim
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
WORKDIR /app
COPY pyproject.toml README.md LICENSE NOTICE ./
COPY atena ./atena
RUN pip install --no-cache-dir . && useradd -r -u 10001 atena
USER atena
EXPOSE 8000
CMD ["atena", "serve", "--host", "0.0.0.0", "--port", "8000"]

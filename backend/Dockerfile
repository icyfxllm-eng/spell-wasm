FROM python:3.12-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
# All backend modules: word API (app.py) + The Climb (auth/climb/db/usernames)
# and the username blocklist data file.
COPY *.py blocklist.txt ./
RUN mkdir -p /data/audio_cache /data/climb
ENV AUDIO_CACHE_DIR=/data/audio_cache
EXPOSE 8000
CMD ["gunicorn", "--bind", "0.0.0.0:8000", "--workers", "2", "app:app"]

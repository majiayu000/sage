# SWE-bench Evaluation Docker Image
FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
RUN pip install --no-cache-dir \
    swebench \
    datasets \
    gitpython

# Create work directory
WORKDIR /eval

# Copy evaluation scripts
COPY run_evaluation.py .
COPY predictions.json .

# Default command
ENTRYPOINT ["python", "run_evaluation.py"]
CMD ["evaluate", "predictions.json"]

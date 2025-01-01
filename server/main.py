from flask import Flask

app = Flask(__name__)

@app.route('/')
def index():
    return "Flask sidecar is running"

if __name__ == '__main__':
    # Run Flask in production mode when launched as sidecar
    app.run(port=5000, host='127.0.0.1')
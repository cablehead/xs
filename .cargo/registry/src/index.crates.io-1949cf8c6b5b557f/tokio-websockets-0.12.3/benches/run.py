import subprocess
import matplotlib.pyplot as plt
from collections import defaultdict
from time import sleep

# Number of samples to consider
NUM_SAMPLES = 10
# Number of warmup samples
NUM_WARMUPS = 2


targets = [
    {"port": 8080, "name": "fastwebsockets", "server": "/usr/bin/fastwebsockets"},
    {"port": 9001, "name": "uWebSockets", "server": "/usr/bin/uWebSockets"},
    {"port": 8080, "name": "tokio-tungstenite", "server": "/usr/bin/tokio-tungstenite"},
    {"port": 3000, "name": "tokio-websockets", "server": "/usr/bin/tokio-websockets"},
]

cases = [
    {"conn": 100, "bytes": 0},
    {"conn": 100, "bytes": 20},
    {"conn": 10, "bytes": 1024},
    {"conn": 10, "bytes": 16 * 1024},
    {"conn": 200, "bytes": 16 * 1024},
    {"conn": 500, "bytes": 16 * 1024},
]


def load_test(conn: int, port: int, bytes_size: int):
    cmd = [
        "stdbuf",
        "-oL",
        "load_test",
        str(conn),
        "0.0.0.0",
        str(port),
        "0",
        "0",
        str(bytes_size),
    ]
    return subprocess.Popen(cmd, stdout=subprocess.PIPE, bufsize=1, text=True)


def run_benchmarks():
    for case in cases:
        conn, bytes_size = case["conn"], case["bytes"]
        results = defaultdict(float)

        for target in targets:
            port, name, server = target["port"], target["name"], target["server"]
            logs = []

            try:
                proc = subprocess.Popen([server])
                print(f"Waiting for {name} to start...")
                sleep(1)

                client = load_test(conn, port, bytes_size)
                count = 0

                for line in client.stdout:
                    if not line:
                        break
                    if not line.startswith("Msg/sec"):
                        continue
                    logs.append(line)
                    if len(logs) == NUM_WARMUPS + NUM_SAMPLES:
                        break

                client.terminate()
                proc.terminate()
                proc.wait()
                client.wait()

            except Exception as e:
                print(f"Error testing {name}: {e}")

            lines = logs[NUM_WARMUPS:]
            mps = [float(line.split()[1].strip()) for line in lines]
            avg = sum(mps) / len(mps) if mps else 0
            results[name] = avg

        results = dict(sorted(results.items(), key=lambda item: item[1], reverse=True))

        title = f"Connections: {conn}, Payload size: {bytes_size}"
        create_chart(title, results, f"{conn}-{bytes_size}-chart.png")


def create_chart(title, results, filename):
    labels = list(results.keys())
    values = list(results.values())

    plt.figure(figsize=(10, 6))
    plt.bar(labels, values, color="blue")
    plt.title(title)
    plt.xlabel("WebSocket Implementations", labelpad=20)
    plt.ylabel("Messages per Second (Avg)", labelpad=20)
    plt.tight_layout()
    plt.savefig(filename)
    plt.close()


if __name__ == "__main__":
    run_benchmarks()

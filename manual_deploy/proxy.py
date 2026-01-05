import socket
import threading
import websocket # pip install websocket-client
import ssl
import sys
import time

# CONFIG
LOCAL_HOST = "127.0.0.1"
LOCAL_PORT = 3333
REMOTE_WS = "wss://voltcore-node.hf.space/stratum"

def handle_client(client_socket):
    print(f"[*] New Miner Connected: {client_socket.getpeername()}")
    
    # Connect to Remote WS
    try:
        ws = websocket.create_connection(REMOTE_WS, sslopt={"cert_reqs": ssl.CERT_NONE})
        print("[*] Connected to Remote Node via WSS")
    except Exception as e:
        print(f"[!] Failed to connect to remote: {e}")
        client_socket.close()
        return

    def ws_to_tcp():
        while True:
            try:
                data = ws.recv()
                if not data: break
                # print(f"<-- {data}")
                client_socket.sendall(data.encode('utf-8') if isinstance(data, str) else data)
                client_socket.sendall(b"\n") # Ensure newline for miner
            except:
                break
        client_socket.close()
        ws.close()

    def tcp_to_ws():
        buffer = b""
        while True:
            try:
                data = client_socket.recv(1024)
                if not data: break
                buffer += data
                while b"\n" in buffer:
                    line, buffer = buffer.split(b"\n", 1)
                    if line:
                        # print(f"--> {line}")
                        ws.send(line.decode('utf-8'))
            except:
                break
        client_socket.close()
        ws.close()

    t1 = threading.Thread(target=ws_to_tcp)
    t2 = threading.Thread(target=tcp_to_ws)
    t1.start()
    t2.start()
    t1.join()
    t2.join()
    print("[*] Miner Disconnected")

def start_server():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind((LOCAL_HOST, LOCAL_PORT))
    server.listen(5)
    print(f"[*] Stratum Bridge Listening on {LOCAL_HOST}:{LOCAL_PORT}")
    print(f"[*] Target: {REMOTE_WS}")
    
    while True:
        client, addr = server.accept()
        threading.Thread(target=handle_client, args=(client,)).start()

if __name__ == "__main__":
    start_server()

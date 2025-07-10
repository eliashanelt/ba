using System;
using System.Net;
using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using System.Collections.Generic;
using Cytosurge;

namespace PressureControllerServer
{
    public class PressureWebSocketServer
    {
        private HttpListener _httpListener;
        private PressureAPI _device;
        private readonly List<WebSocket> _connectedClients = new List<WebSocket>();
        private CancellationTokenSource _cancellationTokenSource;
        private double _minPressure;
        private double _maxPressure;
        private Timer? _pressureMonitorTimer;
        private readonly object _deviceLock = new object();

        public PressureWebSocketServer()
        {
            _device = new PressureAPI();
        }

        public Task StartAsync(string url = "http://localhost:8081/")
        {
            _cancellationTokenSource = new CancellationTokenSource();

            // Initialize pressure device
            InitializePressureDevice();

            // Start HTTP listener
            _httpListener = new HttpListener();
            _httpListener.Prefixes.Add(url);
            _httpListener.Start();

            Console.WriteLine($"Pressure WebSocket Server started at {url}");

            // Accept connections
            _ = Task.Run(async () => await AcceptConnectionsAsync(_cancellationTokenSource.Token));

            return Task.CompletedTask;
        }

        private void InitializePressureDevice()
        {
            try
            {
                _device.Connect(ex =>
                {
                    Console.WriteLine($"Pressure device error: {ex}");
                    BroadcastError($"Device error: {ex.Message}");
                });

                _device.ReadPressureRange(out _minPressure, out _maxPressure);
                Console.WriteLine($"Pressure range: {_minPressure} to {_maxPressure} mbar");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to initialize pressure device: {ex}");
                BroadcastError($"Failed to initialize device: {ex.Message}");
            }
        }

        private async Task AcceptConnectionsAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    var context = await _httpListener.GetContextAsync();

                    if (context.Request.IsWebSocketRequest && context.Request.Url?.AbsolutePath == "/pressure")
                    {
                        // Add CORS headers for WebSocket upgrade
                        context.Response.Headers.Add("Access-Control-Allow-Origin", "*");
                        context.Response.Headers.Add("Access-Control-Allow-Headers", "Content-Type, Accept, X-Requested-With");

                        var wsContext = await context.AcceptWebSocketAsync(null);
                        _ = Task.Run(async () => await HandleWebSocketAsync(wsContext.WebSocket, cancellationToken));
                    }
                    else if (context.Request.HttpMethod == "OPTIONS")
                    {
                        // Handle CORS preflight
                        context.Response.Headers.Add("Access-Control-Allow-Origin", "*");
                        context.Response.Headers.Add("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
                        context.Response.Headers.Add("Access-Control-Allow-Headers", "Content-Type, Accept, X-Requested-With");
                        context.Response.StatusCode = 200;
                        context.Response.Close();
                    }
                    else
                    {
                        context.Response.StatusCode = 400;
                        context.Response.Close();
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Error accepting connection: {ex}");
                }
            }
        }

        private async Task HandleWebSocketAsync(WebSocket webSocket, CancellationToken cancellationToken)
        {
            lock (_connectedClients)
            {
                _connectedClients.Add(webSocket);
            }

            Console.WriteLine("Client connected");

            var buffer = new ArraySegment<byte>(new byte[4096]);

            try
            {
                while (webSocket.State == WebSocketState.Open && !cancellationToken.IsCancellationRequested)
                {
                    var result = await webSocket.ReceiveAsync(buffer, cancellationToken);

                    if (result.MessageType == WebSocketMessageType.Text)
                    {
                        var message = Encoding.UTF8.GetString(buffer.Array!, 0, result.Count);
                        await ProcessMessageAsync(webSocket, message);
                    }
                    else if (result.MessageType == WebSocketMessageType.Close)
                    {
                        break;
                    }
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"WebSocket error: {ex}");
            }
            finally
            {
                lock (_connectedClients)
                {
                    _connectedClients.Remove(webSocket);
                }

                if (webSocket.State == WebSocketState.Open)
                {
                    await webSocket.CloseAsync(WebSocketCloseStatus.NormalClosure, "", CancellationToken.None);
                }

                Console.WriteLine("Client disconnected");
            }
        }

        private async Task ProcessMessageAsync(WebSocket webSocket, string message)
        {
            try
            {
                var jsonDoc = JsonDocument.Parse(message);
                var root = jsonDoc.RootElement;
                var messageType = root.GetProperty("type").GetString();

                switch (messageType)
                {
                    case "ReadPressure":
                        await HandleReadPressure(webSocket);
                        break;

                    case "ReadPressureRange":
                        await HandleReadPressureRange(webSocket);
                        break;

                    case "SetPressure":
                        var pressure = root.GetProperty("pressure").GetDouble();
                        await HandleSetPressure(webSocket, pressure);
                        break;

                    default:
                        await SendMessageAsync(webSocket, new
                        {
                            type = "Error",
                            message = $"Unknown message type: {messageType}"
                        });
                        break;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error processing message: {ex}");
                await SendMessageAsync(webSocket, new
                {
                    type = "Error",
                    message = $"Error processing message: {ex.Message}"
                });
            }
        }

        private async Task HandleReadPressure(WebSocket webSocket)
        {
            return;
            try
            {
                double pressure;
                lock (_deviceLock)
                {
                    pressure = _device.GetPressure();
                }

                await SendMessageAsync(webSocket, new
                {
                    type = "PressureReading",
                    pressure = pressure
                });
            }
            catch (Exception ex)
            {
                await SendMessageAsync(webSocket, new
                {
                    type = "Error",
                    message = $"Failed to read pressure: {ex.Message}"
                });
            }
        }

        private async Task HandleReadPressureRange(WebSocket webSocket)
        {
            await SendMessageAsync(webSocket, new
            {
                type = "PressureRangeReading",
                min = _minPressure,
                max = _maxPressure
            });
        }

        private async Task HandleSetPressure(WebSocket webSocket, double pressure)
        {
            try
            {
                // Validate pressure is within range
                if (pressure < _minPressure || pressure > _maxPressure)
                {
                    await SendMessageAsync(webSocket, new
                    {
                        type = "Error",
                        message = $"Pressure {pressure} is outside valid range [{_minPressure}, {_maxPressure}]"
                    });
                    return;
                }

                lock (_deviceLock)
                {
                    _device.SetPressure(pressure);
                }

                await SendMessageAsync(webSocket, new
                {
                    type = "PressureSet",
                    pressure = pressure
                });

                // Broadcast the new pressure to all connected clients
                await BroadcastPressureUpdate(pressure);
            }
            catch (Exception ex)
            {
                await SendMessageAsync(webSocket, new
                {
                    type = "Error",
                    message = $"Failed to set pressure: {ex.Message}"
                });
            }
        }

        private async Task SendMessageAsync(WebSocket webSocket, object message)
        {
            if (webSocket.State != WebSocketState.Open)
                return;

            var json = JsonSerializer.Serialize(message);
            var bytes = Encoding.UTF8.GetBytes(json);

            try
            {
                await webSocket.SendAsync(
                    new ArraySegment<byte>(bytes),
                    WebSocketMessageType.Text,
                    true,
                    CancellationToken.None
                );
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to send message: {ex}");
            }
        }

        private async Task BroadcastPressureUpdate(double pressure)
        {
            var message = new
            {
                type = "PressureReading",
                pressure = pressure
            };

            var tasks = new List<Task>();

            lock (_connectedClients)
            {
                foreach (var client in _connectedClients)
                {
                    tasks.Add(SendMessageAsync(client, message));
                }
            }

            await Task.WhenAll(tasks);
        }

        private void BroadcastError(string errorMessage)
        {
            var message = new
            {
                type = "Error",
                message = errorMessage
            };

            lock (_connectedClients)
            {
                foreach (var client in _connectedClients)
                {
                    _ = SendMessageAsync(client, message);
                }
            }
        }

        public void Stop()
        {
            _cancellationTokenSource?.Cancel();
            _pressureMonitorTimer?.Dispose();
            _httpListener?.Stop();
            _httpListener?.Close();

            // Close all client connections
            lock (_connectedClients)
            {
                foreach (var client in _connectedClients)
                {
                    try
                    {
                        client.CloseAsync(WebSocketCloseStatus.NormalClosure, "Server shutting down", CancellationToken.None).Wait();
                    }
                    catch { }
                }
                _connectedClients.Clear();
            }

            _device?.Dispose();
        }
    }

    class Program
    {
        static async Task Main(string[] args)
        {
            var server = new PressureWebSocketServer();

            try
            {
                await server.StartAsync("http://localhost:8081/");

                Console.WriteLine("Press any key to stop the server...");
                Console.ReadKey();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Server error: {ex}");
            }
            finally
            {
                server.Stop();
            }
        }
    }
}
// See https://aka.ms/new-console-template for more information
using Cytosurge;

PressureAPI device = new PressureAPI();
Console.WriteLine("Hello, World!");
device.Connect(ex => Console.WriteLine($"The device has experienced a runtime error: ex.ToString()"));
device.ReadPressureRange(out var min, out var max);
Console.WriteLine($"Allowed range [min: {min}; max: {max}] mbar");
Console.WriteLine($"Current pressure: {device.GetPressure()} mbar");
device.SetPressure(200.0);
Console.WriteLine($"Current pressure: {device.GetPressure()} mbar");
Thread.Sleep(5000);
device.SetPressure(300.0);
Thread.Sleep(5000);
device.SetPressure(0.0);

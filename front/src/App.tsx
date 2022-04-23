import { useEffect, useRef, useState } from "react";
import { GithubPicker, RGBColor } from "react-color";
import { ToastContainer, toast } from "react-toastify";

import "./App.css";
import "react-toastify/dist/ReactToastify.css";

const GRID_SIZE = 90;
const GRID_CELL_SIZE = 10;
const GRID_PIXEL = GRID_CELL_SIZE * GRID_SIZE;

function pixel_pos_to_grid_pos(x: number, y: number) {
  return {
    x: Math.trunc(x / GRID_CELL_SIZE),
    y: Math.trunc(y / GRID_CELL_SIZE),
  };
}

const devmode = process.env.NODE_ENV === "development";
const server_address = devmode
  ? "localhost:8080"
  : "bplace-api.preview.blackfoot.dev";
const secure = !devmode;

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [username, setUsername] = useState<string | null | undefined>(
    localStorage.getItem("username")
  );
  const [currentWs, setCurrentWs] = useState<WebSocket>();
  const [username_input, setUsername_input] = useState<string>();
  const [color, setColor] = useState<RGBColor>({ r: 0, g: 0, b: 0 });

  useEffect(() => {
    if (!username) return;
    let ws: WebSocket;
    if (!currentWs) {
      ws = new WebSocket(
        `ws${secure ? "s" : ""}://${server_address}/ws/${username}`
      );
      setCurrentWs(ws);
    } else {
      ws = currentWs;
    }
    ws.onopen = async () => {
      console.log("ws opened");
      localStorage.setItem("username", username);
      if (canvasRef.current) {
        let query = await fetch(
          `http${secure ? "s" : ""}://${server_address}/`
        );
        let canvas_pixels = await query.json();
        for (const pixel of canvas_pixels) {
          setPixel(pixel, canvasRef.current);
        }
      }
    };
    ws.onclose = (ev: CloseEvent) =>
      console.log(
        toast("WebSocket close, refresh the page", { autoClose: false }),
        ev
      );
    ws.onmessage = (e) => {
      const message = JSON.parse(e.data);
      if (message.error) {
        toast(message.error);
      } else if (canvasRef.current) setPixel(message, canvasRef.current);
    };
  }, [username, canvasRef.current]);

  function setPixel(pixel: any, canvas: HTMLCanvasElement) {
    let canvas2d = canvas.getContext("2d");
    if (canvas2d) {
      let { r, g, b } = pixel.color;
      let { x, y } = pixel.position;
      canvas2d.fillStyle = `rgba(${r}, ${g}, ${b}, 1)`;
      canvas2d.fillRect(x * GRID_CELL_SIZE, y * GRID_CELL_SIZE, 10, 10);
    }
  }

  if (!username) {
    return (
      <div className="centered">
        Please insert your username
        <br />
        <input
          type="text"
          onChange={(event) => {
            setUsername_input(event.currentTarget.value);
          }}
        />
        <button onClick={() => setUsername(username_input)}>Save</button>
      </div>
    );
  }

  return (
    <div className="App">
      <ToastContainer
        position="top-left"
        autoClose={5000}
        hideProgressBar={false}
        newestOnTop
        closeOnClick={false}
        rtl={false}
        pauseOnFocusLoss
        draggable={false}
        pauseOnHover
      />
      <GithubPicker
        color={"#000"}
        colors={[
          "#000",
          "#FFF",
          "#F00",
          "#0F0",
          "#00F",
          "#FF0",
          "#0FF",
          "#F0F",
        ]}
        onChangeComplete={(newColor) => {
          setColor(newColor.rgb);
        }}
        className="color-picker"
      />
      <canvas
        style={{ border: "1px solid black" }}
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          const x = Math.trunc(event.clientX - rect.left);
          const y = Math.trunc(event.clientY - rect.top);
          const { x: grid_x, y: grid_y } = pixel_pos_to_grid_pos(x, y);
          const canvas_ctx = event.currentTarget.getContext("2d");
          const pix = canvas_ctx?.getImageData(x, y, 1, 1).data;

          if (pix) {
            // If white selected and canvas pix is alpha 0
            if (
              pix[3] === 0 &&
              color.r === 255 &&
              color.g === 255 &&
              color.b === 255
            )
              return;
            if (
              pix[3] !== 0 &&
              color.r === pix[0] &&
              color.g === pix[1] &&
              color.b === pix[2]
            )
              return;
          }
          const packet = JSON.stringify({
            position: { x: grid_x, y: grid_y },
            color: color,
          });

          if (currentWs) {
            console.log("sending:", packet);
            currentWs.send(packet);
          }
        }}
        width={GRID_PIXEL}
        height={GRID_PIXEL}
        ref={canvasRef}
      />
    </div>
  );
}

export default App;

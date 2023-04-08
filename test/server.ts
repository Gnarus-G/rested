// @deno-types="npm:@types/express@4.17.15"
import express from "npm:express@4.18.2";

const app = express();

app.use((_, __, next) => {
  console.log("request at", new Date());
  next();
});

app.get("/", (req, res) => {
  res.send(`Welcome to the Dinosaur API! ${req.header("random")}`);
});

const PORT = 8080;

console.log("Listening on port:", PORT);

app.listen(PORT);

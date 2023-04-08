// @deno-types="npm:@types/express@4.17.15"
import express from "npm:express@4.18.2";

const app = express();

app.get("/", (req, res) => {
  res.send(`Welcome to the Dinosaur API! ${req.header("random")}`);
});

app.listen(8080);

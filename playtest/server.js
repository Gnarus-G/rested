import express, { json } from "express";

const app = express();

app.use(json());

app.use((_, __, next) => {
  console.log("request at", new Date());
  next();
});

app.get("/", (req, res) => {
  res.send(`Welcome to the Dinosaur API! ${req.header("random")}`);
});

app.post("/echo", (req, res) => {
  console.log("req body", req.body);
  res.send({
    data: req.body,
  });
});

const PORT = 8080;

console.log("Listening on port:", PORT);

app.listen(PORT);

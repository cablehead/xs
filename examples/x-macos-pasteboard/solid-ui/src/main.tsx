/* @refresh reload */
import "./index.css";
import { render } from "solid-js/web";
import App from "./App.tsx";

render(() => <App />, document.getElementsByTagName("main")[0] as HTMLElement);

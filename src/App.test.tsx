import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the Registry Manager app shell", () => {
    render(<App />);

    expect(screen.getByTestId("app-root")).toBeVisible();
    expect(screen.getByText("Registry Manager")).toBeInTheDocument();
  });
});

import React from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import * as monitoring from "@/lib/monitoring";

describe("ErrorBoundary", () => {
  it("renders fallback, reports error, and can retry", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const reportSpy = vi
      .spyOn(monitoring, "reportRenderError")
      .mockImplementation(() => undefined);

    let shouldThrow = true;

    function Boom() {
      if (shouldThrow) {
        throw new Error("boom");
      }
      return React.createElement("div", null, "OK");
    }

    render(
      React.createElement(
        MemoryRouter,
        null,
        React.createElement(ErrorBoundary, null, React.createElement(Boom)),
      ),
    );

    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("boom")).toBeInTheDocument();
    expect(reportSpy).toHaveBeenCalled();

    shouldThrow = false;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(screen.getByText("OK")).toBeInTheDocument();
  });
});

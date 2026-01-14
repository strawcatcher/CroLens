import { describe, expect, it, vi } from "vitest";
import {
  setupMonitoring,
  reportApiError,
  reportRenderError,
  reportUnhandledRejection,
} from "@/lib/monitoring";

describe("monitoring", () => {
  it("logs api errors as warnings once enabled", () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const errorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    setupMonitoring();
    reportApiError(new Error("boom"));

    expect(warnSpy).toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
  });

  it("logs render errors as errors once enabled", () => {
    const errorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    setupMonitoring();
    reportRenderError(new Error("render"));

    expect(errorSpy).toHaveBeenCalled();
  });

  it("logs unhandled rejections as errors once enabled", () => {
    const errorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    setupMonitoring();
    reportUnhandledRejection("oops");

    expect(errorSpy).toHaveBeenCalled();
  });
});


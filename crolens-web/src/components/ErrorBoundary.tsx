import * as React from "react";
import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { reportRenderError } from "@/lib/monitoring";

type ErrorBoundaryState = {
  error: Error | null;
  errorId: number;
};

export type ErrorBoundaryProps = {
  children: React.ReactNode;
  title?: string;
  description?: string;
};

export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null, errorId: 0 };

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("[ErrorBoundary] Render error:", error, info);
    reportRenderError(error, info.componentStack);
  }

  private reset = () => {
    this.setState((prev) => ({ error: null, errorId: prev.errorId + 1 }));
  };

  render() {
    if (!this.state.error) {
      return (
        <React.Fragment key={this.state.errorId}>
          {this.props.children}
        </React.Fragment>
      );
    }

    const title = this.props.title ?? "Something went wrong";
    const description =
      this.props.description ??
      "A rendering error occurred. You can retry, or go back to the Playground.";

    return (
      <div className="flex min-h-[60vh] items-center justify-center p-6">
        <Card className="w-full max-w-xl">
          <CardHeader>
            <CardTitle>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="rounded-md border border-border bg-[var(--code-bg)] p-3 text-sm text-muted-foreground">
              <div className="font-mono">{this.state.error.message}</div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button type="button" onClick={this.reset}>
                Retry
              </Button>
              <Button type="button" variant="outline" asChild>
                <Link to="/playground">Go to Playground</Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }
}

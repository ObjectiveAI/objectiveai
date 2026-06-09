import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { describe, it, expect, vi, afterEach } from "vitest";
import { EmailSignup } from "./EmailSignup";

vi.mock("@/app/actions/subscribe", () => ({
  subscribe: vi.fn(),
}));

import { subscribe } from "@/app/actions/subscribe";
const mockSubscribe = vi.mocked(subscribe);

describe("EmailSignup", () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders the form with input and button", () => {
    render(<EmailSignup />);
    expect(screen.getByPlaceholderText("your email")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "notify me" })).toBeInTheDocument();
  });

  it("shows validation error for invalid email", async () => {
    render(<EmailSignup />);
    const input = screen.getByPlaceholderText("your email");
    const button = screen.getByRole("button", { name: "notify me" });

    fireEvent.change(input, { target: { value: "not-an-email" } });
    fireEvent.click(button);

    expect(screen.getByText("enter a valid email")).toBeInTheDocument();
  });

  it("clears error when user types after validation failure", async () => {
    render(<EmailSignup />);
    const input = screen.getByPlaceholderText("your email");
    const button = screen.getByRole("button", { name: "notify me" });

    fireEvent.click(button);
    expect(screen.getByText("enter a valid email")).toBeInTheDocument();

    fireEvent.change(input, { target: { value: "a" } });
    expect(screen.queryByText("enter a valid email")).not.toBeInTheDocument();
  });

  it("submits and shows success", async () => {
    mockSubscribe.mockResolvedValue({ ok: true });

    render(<EmailSignup />);
    const input = screen.getByPlaceholderText("your email");
    const button = screen.getByRole("button", { name: "notify me" });

    fireEvent.change(input, { target: { value: "test@example.com" } });
    fireEvent.click(button);

    expect(screen.getByRole("button")).toHaveTextContent("...");

    await vi.waitFor(() => {
      expect(screen.getByText("you're on the list")).toBeInTheDocument();
    });

    expect(mockSubscribe).toHaveBeenCalledWith("test@example.com");
  });

  it("shows error on failure", async () => {
    mockSubscribe.mockResolvedValue({ ok: false, error: "something went wrong" });

    render(<EmailSignup />);
    fireEvent.change(screen.getByPlaceholderText("your email"), {
      target: { value: "test@example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "notify me" }));

    await vi.waitFor(() => {
      expect(screen.getByText("something went wrong")).toBeInTheDocument();
    });
  });
});

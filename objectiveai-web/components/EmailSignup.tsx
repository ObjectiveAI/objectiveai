"use client";

import { useState, type FormEvent } from "react";

interface EmailSignupProps {
  /** CSS class for the <form> element */
  formClassName?: string;
  /** CSS class for the <input> element */
  inputClassName?: string;
  /** CSS class for the <button> element */
  buttonClassName?: string;
  /** CSS class for the success <p> */
  confirmationClassName?: string;
  /** CSS class for the error <p> */
  errorClassName?: string;
}

export function EmailSignup({
  formClassName,
  inputClassName,
  buttonClassName,
  confirmationClassName,
  errorClassName,
}: EmailSignupProps) {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<"idle" | "submitting" | "success" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState("something went wrong");

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (state === "submitting") return;

    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setState("error");
      setErrorMsg("enter a valid email");
      return;
    }

    setState("submitting");
    try {
      const res = await fetch(
        "https://buttondown.com/api/emails/embed-subscribe/objectiveai",
        {
          method: "POST",
          headers: { "Content-Type": "application/x-www-form-urlencoded" },
          body: new URLSearchParams({ email }),
        }
      );
      if (res.ok || res.status === 303) {
        setState("success");
        setEmail("");
      } else {
        setErrorMsg("something went wrong");
        setState("error");
      }
    } catch {
      setErrorMsg("something went wrong");
      setState("error");
    }
  }

  return (
    <>
      <form className={formClassName} onSubmit={handleSubmit} noValidate>
        <input
          type="email"
          className={inputClassName}
          placeholder="your email"
          value={email}
          onChange={(e) => { setEmail(e.target.value); if (state === "error") setState("idle"); }}
          required
        />
        <button
          type="submit"
          className={buttonClassName}
          disabled={state === "submitting"}
        >
          {state === "submitting" ? "..." : "notify me"}
        </button>
      </form>
      {state === "success" && <p className={confirmationClassName}>you&apos;re on the list</p>}
      {state === "error" && <p className={errorClassName}>{errorMsg}</p>}
    </>
  );
}

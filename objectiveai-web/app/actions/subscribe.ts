"use server";

export async function subscribe(email: string): Promise<{ ok: boolean; error?: string }> {
  if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    return { ok: false, error: "enter a valid email" };
  }

  const apiKey = process.env.BUTTONDOWN_API_KEY;
  if (!apiKey) {
    return { ok: false, error: "something went wrong" };
  }

  try {
    const res = await fetch("https://api.buttondown.com/v1/subscribers", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Token ${apiKey}`,
      },
      body: JSON.stringify({ email_address: email }),
    });

    if (res.ok || res.status === 201) {
      return { ok: true };
    }

    const body = await res.json().catch(() => null);
    if (res.status === 409 || body?.email_address?.[0]?.includes("already")) {
      return { ok: true };
    }

    return { ok: false, error: "something went wrong" };
  } catch {
    return { ok: false, error: "something went wrong" };
  }
}

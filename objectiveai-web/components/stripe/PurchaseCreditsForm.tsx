"use client";

import { useState, useEffect, useCallback } from "react";
import {
  Elements,
  PaymentElement,
  useStripe,
  useElements,
} from "@stripe/react-stripe-js";
import { StripeAddressElementChangeEvent } from "@stripe/stripe-js";
import { useObjectiveAI } from "@/hooks/useObjectiveAI";
import { useStripeTheme } from "@/hooks/useStripeTheme";
import { useIsMobile } from "@/hooks/useIsMobile";
import { CustomerSession, PaymentIntentPreview, PaymentIntent } from "@/lib/stripe-types";
import {
  stripePromise,
  customerSessionToBillingAddress,
  getStripeAppearance,
  withStripeRetry,
} from "@/lib/stripe";
import {
  STRIPE_MIN_CREDITS_CENTS,
  STRIPE_MAX_CREDITS_CENTS,
} from "@/lib/constants";
import { CreditsBreakdown } from "./CreditsBreakdown";

interface PurchaseCreditsFormProps {
  onClose: () => void;
  onNeedBillingAddress: () => void;
  onSuccess?: () => void;
}

/**
 * Form for purchasing credits with Stripe payment.
 * Two-phase flow: preview → confirm with payment.
 */
export function PurchaseCreditsForm({
  onClose,
  onNeedBillingAddress,
  onSuccess,
}: PurchaseCreditsFormProps) {
  const { getClient } = useObjectiveAI();
  const theme = useStripeTheme();
  const isMobile = useIsMobile();

  // Customer session
  const [customerSession, setCustomerSession] = useState<CustomerSession | null>();
  const [, setBillingAddress] =
    useState<StripeAddressElementChangeEvent["value"]>();

  // Amount in cents
  const [amountCents, setAmountCents] = useState<number | undefined>(500);
  const [amountInputValue, setAmountInputValue] = useState("5");
  const [amountValid, setAmountValid] = useState(true);

  // Preview
  const [paymentIntentPreview, setPaymentIntentPreview] =
    useState<PaymentIntentPreview>();

  // Confirmed state (payment intent created with tax)
  const [confirming, setConfirming] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [paymentIntent, setPaymentIntent] = useState<PaymentIntent>();

  // Error state
  const [error, setError] = useState<string | null>(null);

  // Fetch customer session on mount
  useEffect(() => {
    let cancelled = false;

    const fetchCustomerSession = async () => {
      try {
        const client = await getClient();
        const session = await client.get_unary<CustomerSession | null>(
          "/stripe/customer"
        );

        if (cancelled) return;

        if (session === null) {
          onNeedBillingAddress();
        } else {
          setCustomerSession((prev) => prev ?? session);
          setBillingAddress(customerSessionToBillingAddress(session));
        }
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : "Failed to load customer info");
      }
    };

    fetchCustomerSession();

    return () => {
      cancelled = true;
    };
  }, [getClient, onNeedBillingAddress]);

  // Fetch preview when amount changes
  useEffect(() => {
    let cancelled = false;

    const fetchPreview = async () => {
      if (
        amountCents === undefined ||
        amountCents < STRIPE_MIN_CREDITS_CENTS ||
        amountCents > STRIPE_MAX_CREDITS_CENTS
      ) {
        setPaymentIntentPreview(undefined);
        return;
      }

      try {
        const client = await getClient();
        const preview = await client.post_unary<PaymentIntentPreview>(
          "/stripe/payment_intent",
          { amount_cents: amountCents }
        );

        if (cancelled) return;
        setPaymentIntentPreview(preview);
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : "Failed to calculate preview");
      }
    };

    setPaymentIntentPreview(undefined);
    fetchPreview();

    return () => {
      cancelled = true;
    };
  }, [getClient, amountCents]);

  // Handle amount input change
  const handleAmountChange = useCallback((newValue: string) => {
    const match = newValue.match(/^(\d*\.?\d{0,2})\d*$/)?.[1];
    const matchNumber =
      match && match !== "" && match !== "."
        ? Math.round(parseFloat(match) * 100)
        : undefined;

    if (
      matchNumber &&
      matchNumber >= STRIPE_MIN_CREDITS_CENTS &&
      matchNumber <= STRIPE_MAX_CREDITS_CENTS
    ) {
      setAmountInputValue(match!);
      setAmountValid(true);
      setAmountCents(matchNumber);
    } else {
      setAmountInputValue(newValue);
      setAmountValid(false);
      setAmountCents(undefined);
    }
  }, []);

  // Handle confirm (creates payment intent with tax)
  const handleConfirm = async () => {
    if (!paymentIntentPreview || !amountCents) return;

    setConfirming(true);
    setError(null);

    try {
      const client = await getClient();

      const intent = await withStripeRetry(async () => {
        return await client.post_unary<PaymentIntent>("/stripe/payment_intent", {
          amount_cents: amountCents,
          save_payment_method: true,
          confirm: true,
        });
      });

      // Re-fetch customer session
      const session = await client.get_unary<CustomerSession | null>(
        "/stripe/customer"
      );

      if (session === null) {
        onNeedBillingAddress();
      } else {
        setPaymentIntent(intent);
        setBillingAddress(customerSessionToBillingAddress(session));
        setConfirmed(true);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to confirm payment");
    } finally {
      setConfirming(false);
    }
  };

  // Loading state
  if (customerSession === undefined) {
    return (
      <div
        className="card"
        style={{
          padding: isMobile ? "24px 16px" : "32px",
          textAlign: "center",
        }}
      >
        <div
          style={{
            width: "32px",
            height: "32px",
            border: "3px solid var(--border)",
            borderTopColor: "var(--accent)",
            borderRadius: "50%",
            animation: "spin 1s linear infinite",
            margin: "0 auto 16px",
          }}
        />
        <p style={{ color: "var(--text-muted)", fontSize: "14px" }}>
          Loading payment information...
        </p>
      </div>
    );
  }

  return (
    <div className="card" style={{ padding: isMobile ? "24px 16px" : "32px" }}>
      <h3
        style={{
          fontSize: "18px",
          fontWeight: 600,
          marginBottom: "20px",
        }}
      >
        Purchase Credits
      </h3>

      {error && (
        <div
          style={{
            padding: "12px 16px",
            background: "rgba(239, 68, 68, 0.1)",
            border: "1px solid rgba(239, 68, 68, 0.3)",
            borderRadius: "8px",
            color: "var(--color-error)",
            fontSize: "14px",
            marginBottom: "20px",
          }}
        >
          {error}
        </div>
      )}

      {/* Amount Input */}
      <div style={{ marginBottom: "24px" }}>
        <label
          style={{
            display: "block",
            fontSize: "14px",
            fontWeight: 500,
            marginBottom: "8px",
            color: "var(--text-muted)",
          }}
        >
          Amount (USD)
        </label>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            border: `1px solid ${amountValid ? "var(--border)" : "var(--color-error)"}`,
            borderRadius: "8px",
            overflow: "hidden",
          }}
        >
          <span
            style={{
              padding: "12px 16px",
              background: "var(--nav-surface)",
              borderRight: "1px solid var(--border)",
              color: "var(--text-muted)",
              fontWeight: 500,
            }}
          >
            $
          </span>
          <input
            type="text"
            value={amountInputValue}
            onChange={(e) => handleAmountChange(e.target.value)}
            disabled={confirming || confirmed}
            style={{
              flex: 1,
              padding: "12px 16px",
              border: "none",
              outline: "none",
              fontSize: "16px",
              fontFamily: "monospace",
              background: "var(--card-bg)",
              color: "var(--text)",
              cursor: confirming || confirmed ? "not-allowed" : "text",
            }}
          />
        </div>
        {!amountValid && (
          <p
            style={{
              fontSize: "12px",
              color: "var(--color-error)",
              marginTop: "4px",
            }}
          >
            Amount must be between $5 and $10,000
          </p>
        )}
      </div>

      {/* Breakdown */}
      <div style={{ marginBottom: "24px" }}>
        <CreditsBreakdown
          paymentIntentPreview={paymentIntentPreview}
          paymentIntent={paymentIntent}
          confirmingOrConfirmed={confirming || confirmed}
        />
      </div>

      {/* Payment Element (after confirm) */}
      {confirmed && customerSession && paymentIntent && (
        <PaymentSection
          customerSession={customerSession}
          paymentIntent={paymentIntent}
          theme={theme}
          onClose={onClose}
          onSuccess={onSuccess}
        />
      )}

      {/* Continue Button (before confirm) */}
      {!confirmed && (
        <button
          onClick={handleConfirm}
          disabled={!paymentIntentPreview || confirming}
          style={{
            width: "100%",
            padding: "14px 24px",
            fontSize: "15px",
            fontWeight: 600,
            background:
              paymentIntentPreview && !confirming ? "var(--accent)" : "var(--border)",
            color:
              paymentIntentPreview && !confirming ? "white" : "var(--text-muted)",
            border: "none",
            borderRadius: "8px",
            cursor:
              paymentIntentPreview && !confirming ? "pointer" : "not-allowed",
          }}
        >
          {confirming ? "Processing..." : "Continue"}
        </button>
      )}
    </div>
  );
}

/**
 * Payment section with Stripe PaymentElement.
 * Separated to use Stripe hooks.
 */
function PaymentSection({
  customerSession,
  paymentIntent,
  theme,
  onClose,
  onSuccess,
}: {
  customerSession: CustomerSession;
  paymentIntent: PaymentIntent;
  theme: "light" | "dark";
  onClose: () => void;
  onSuccess?: () => void;
}) {
  const [elementsReady, setElementsReady] = useState(false);

  return (
    <Elements
      key={paymentIntent.client_secret}
      stripe={stripePromise}
      options={{
        appearance: getStripeAppearance(theme),
        customerSessionClientSecret: customerSession.client_secret,
        clientSecret: paymentIntent.client_secret,
      }}
    >
      <PaymentElement
        options={{ layout: "tabs" }}
        onReady={() => setElementsReady(true)}
      />
      {elementsReady && (
        <PaymentSubmitButton onClose={onClose} onSuccess={onSuccess} />
      )}
    </Elements>
  );
}

/**
 * Submit button that uses Stripe hooks.
 */
function PaymentSubmitButton({
  onClose,
  onSuccess,
}: {
  onClose: () => void;
  onSuccess?: () => void;
}) {
  const stripe = useStripe();
  const elements = useElements();
  const [purchasing, setPurchasing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handlePurchase = async () => {
    if (!stripe || !elements) return;

    setPurchasing(true);
    setError(null);

    const { selectedPaymentMethod, error: elementsError } =
      await elements.submit();

    if (elementsError) {
      setError(elementsError.message || "Payment failed");
      setPurchasing(false);
      return;
    }

    const { error: paymentError } = await stripe.confirmPayment({
      elements,
      confirmParams: {
        return_url: `${window.location.origin}/account/credits`,
        payment_method: selectedPaymentMethod,
      },
    });

    if (paymentError) {
      setError(paymentError.message || "Payment failed");
      setPurchasing(false);
      return;
    }

    // Success - redirect will happen via return_url
    onSuccess?.();
    onClose();
  };

  return (
    <div style={{ marginTop: "24px" }}>
      {error && (
        <div
          style={{
            padding: "12px 16px",
            background: "rgba(239, 68, 68, 0.1)",
            border: "1px solid rgba(239, 68, 68, 0.3)",
            borderRadius: "8px",
            color: "var(--color-error)",
            fontSize: "14px",
            marginBottom: "16px",
          }}
        >
          {error}
        </div>
      )}
      <button
        onClick={handlePurchase}
        disabled={!stripe || purchasing}
        style={{
          width: "100%",
          padding: "14px 24px",
          fontSize: "15px",
          fontWeight: 600,
          background: stripe && !purchasing ? "var(--accent)" : "var(--border)",
          color: stripe && !purchasing ? "white" : "var(--text-muted)",
          border: "none",
          borderRadius: "8px",
          cursor: stripe && !purchasing ? "pointer" : "not-allowed",
        }}
      >
        {purchasing ? "Processing..." : "Purchase"}
      </button>
    </div>
  );
}

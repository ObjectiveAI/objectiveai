"use client";

import {
  loadStripe,
  Stripe,
  StripeAddressElementChangeEvent,
  StripeElements,
} from "@stripe/stripe-js";
import {
  PaymentElement,
  Elements,
  useStripe,
  useElements,
  AddressElement,
} from "@stripe/react-stripe-js";
import { Provider } from "@/provider";
import { ReactElement, ReactNode, useEffect, useMemo, useState } from "react";
import cn from "classnames";
import { openAi } from "@/components/Common";
import { SharedFooter } from "../../SharedFooter";
import { SharedHeader } from "../../SharedHeader";
import { LoadingDots } from "@/components/Loading";
import { APIError } from "openai";
import { costFormatter } from "@/format";
import { Auth } from "objectiveai";
import { useTheme } from "@/theme";

const stripePublishableKey =
  process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY ??
  (() => {
    throw new Error("NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY is not defined");
  })();
const apiUrl =
  process.env.NEXT_PUBLIC_API_URL ??
  (() => {
    throw new Error("NEXT_PUBLIC_API_URL is not defined");
  })();

interface PaymentIntentPreview {
  credits_amount_cents: number;
  fee_amount_cents: number;
  subtotal_amount_cents: number;
}

interface PaymentIntent extends PaymentIntentPreview {
  client_secret: string;
  tax_amount_cents: number;
  total_amount_cents: number;
}

interface CustomerSession {
  client_secret: string;
  customer: {
    id: string;
    name: string | null;
    address: {
      city: string | null;
      country: string | null;
      line1: string | null;
      line2: string | null;
      postal_code: string | null;
      state: string | null;
    } | null;
  };
}

export function Credits({
  session,
  className,
}: {
  session: Provider.TokenSession;
  className?: string;
}): ReactElement {
  const stripe = useMemo(() => loadStripe(stripePublishableKey), []);
  const [mode, setMode] = useState<null | "address" | "purchase">(null);
  return (
    <main className={cn("h-[100dvh]", "flex", "flex-col", className)}>
      <SharedHeader session={session} />
      <div className={cn("flex-grow", "overflow-y-auto")}>
        <div
          className={cn(
            "w-[calc(var(--spacing)*192)]",
            "max-w-[calc(100%-var(--spacing)*4)]",
            "whitespace-nowrap",
            "mx-auto",
            "space-y-3",
            "mt-4"
          )}
        >
          <AvailableCredits session={session} />
          <div className={cn("flex", "w-full", "gap-3")}>
            <StripeButton
              title="Purchase Credits"
              open={mode === "purchase"}
              onToggleOpen={() =>
                setMode(mode === "purchase" ? null : "purchase")
              }
            />
            <StripeButton
              title="Billing Address"
              open={mode === "address"}
              onToggleOpen={() =>
                setMode(mode === "address" ? null : "address")
              }
            />
          </div>
          {mode === "address" && (
            <BillingAddress
              stripe={stripe}
              session={session}
              onClose={() => setMode(null)}
            />
          )}
          {mode === "purchase" && (
            <PurchaseCredits
              stripe={stripe}
              session={session}
              onClose={() => setMode(null)}
              onNeedBillingAddress={() => setMode("address")}
            />
          )}
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}

function customerSessionToBillingAddress({
  customer: { name, address },
}: CustomerSession): StripeAddressElementChangeEvent["value"] {
  return {
    name: name ?? "",
    address: {
      line1: address?.line1 ?? "",
      line2: address?.line2 ?? null,
      city: address?.city ?? "",
      state: address?.state ?? "",
      postal_code: address?.postal_code ?? "",
      country: address?.country ?? "",
    },
  };
}

function StripeButton({
  title,
  open,
  onToggleOpen,
  className,
}: {
  title: string;
  open?: boolean;
  onToggleOpen: () => void;
  className?: string;
}): ReactElement {
  return (
    <button
      className={cn(
        "flex",
        "flex-grow",
        "items-center",
        "justify-center",
        "px-4",
        "py-2",
        "border",
        "rounded-lg",
        "cursor-pointer",
        open
          ? cn("border-highlight-secondary", "bg-highlight-secondary")
          : cn(
              "border-muted-secondary",
              "hover:border-highlight-muted",
              "hover:bg-highlight-muted"
            ),
        "transition-colors",
        "duration-200",
        className
      )}
      onClick={onToggleOpen}
    >
      {title}
    </button>
  );
}

function AvailableCredits({
  session,
  className,
}: {
  session: Provider.TokenSession;
  className?: string;
}): ReactElement {
  const [credits, setCredits] = useState<Auth.Credits>();
  const creditsFormatted = useMemo(
    () =>
      credits !== undefined
        ? costFormatter(6).format(credits.credits)
        : undefined,
    [credits]
  );

  useEffect(() => {
    (async () => {
      const openai = await openAi(session);
      const credits = await Auth.Credits.retrieve(openai);
      setCredits(credits);
    })();
  }, [session]);

  return (
    <div
      className={cn(
        "flex",
        "justify-center",
        "items-center",
        "p-4",
        "border",
        "border-muted-secondary",
        "rounded-lg",
        "text-3xl",
        className
      )}
    >
      {creditsFormatted === undefined ? <LoadingDots /> : creditsFormatted}
    </div>
  );
}

function BillingAddress({
  stripe,
  session,
  onClose,
}: {
  stripe: Promise<Stripe | null>;
  session: Provider.TokenSession;
  onClose: () => void;
}): ReactElement {
  const { theme = "light" } = useTheme();

  // billing address
  const [billingAddress, setBillingAddress] =
    useState<StripeAddressElementChangeEvent["value"]>();

  // customer session
  const [customerSession, setCustomerSession] = useState<
    [CustomerSession, StripeAddressElementChangeEvent["value"]] | null
  >();
  useEffect(() => {
    setCustomerSession(undefined);
    setBillingAddress(undefined);
    (async () => {
      const openai = await openAi(session);
      const customerSession = (await openai.get(
        `${apiUrl}/stripe/customer`
      )) as CustomerSession | null;
      if (customerSession) {
        const customerSessionBillingAddress =
          customerSessionToBillingAddress(customerSession);
        setCustomerSession([customerSession, customerSessionBillingAddress]);
        setBillingAddress(customerSessionBillingAddress);
      } else {
        setCustomerSession(null);
        setBillingAddress(undefined);
      }
    })();
  }, [session]);

  // submit new billing address
  const [submitting, setSubmitting] = useState(false);
  const handleSubmit = async () => {
    if (billingAddress === undefined) return;
    setSubmitting(true);
    const openai = await openAi(session);
    let attempt = 0;
    while (true) {
      try {
        await openai.post(`${apiUrl}/stripe/customer`, {
          body: { name: billingAddress.name, address: billingAddress.address },
        });
        break;
      } catch (error) {
        if (error instanceof APIError && error.status === 429 && attempt < 1) {
          attempt += 1;
          // sleep for 10 seconds
          await new Promise((resolve) => setTimeout(resolve, 10000));
        } else {
          throw error;
        }
      }
    }
    onClose();
    setSubmitting(false);
  };

  // Element ready
  const [ready, setReady] = useState(false);

  if (customerSession === undefined) {
    return <LoadingDotsFull />;
  } else {
    return (
      <Elements
        stripe={stripe}
        options={{
          customerSessionClientSecret: customerSession?.[0].client_secret,
          appearance: { theme: theme === "light" ? "stripe" : "night" },
          mode: "setup",
          currency: "usd",
        }}
      >
        <AddressElement
          onChange={(e) => setBillingAddress(e.value)}
          options={{
            mode: "billing",
            defaultValues: { ...customerSession?.[1] },
          }}
          onReady={() => setReady(true)}
        />
        {ready && (
          <SubmitButtonInner
            onClick={handleSubmit}
            disabled={!billingAddress}
            loading={submitting}
          >
            Submit
          </SubmitButtonInner>
        )}
      </Elements>
    );
  }
}

function PurchaseCredits({
  stripe,
  session,
  onClose,
  onNeedBillingAddress,
}: {
  stripe: Promise<Stripe | null>;
  session: Provider.TokenSession;
  onClose: () => void;
  onNeedBillingAddress: () => void;
}): ReactElement {
  const { theme = "light" } = useTheme();

  // customer session billing address
  const [_billingAddress, setBillingAddress] =
    useState<StripeAddressElementChangeEvent["value"]>();

  // customer session
  const [customerSession, setCustomerSession] =
    useState<CustomerSession | null>();
  useEffect(() => {
    setBillingAddress(undefined);
    (async () => {
      const openai = await openAi(session);
      const customerSession = (await openai.get(
        `${apiUrl}/stripe/customer`
      )) as CustomerSession | null;
      if (customerSession === null) {
        onNeedBillingAddress();
      } else {
        setCustomerSession((prev) => prev ?? customerSession);
        setBillingAddress(customerSessionToBillingAddress(customerSession));
      }
    })();
  }, [session, onNeedBillingAddress]);

  // credit purchase amount in USD cents
  const [amountCents, setAmountCents] = useState<number | undefined>(500);

  // payment intent preview
  const [paymentIntentPreview, setPaymentIntentPreview] =
    useState<PaymentIntentPreview>();
  useEffect(() => {
    setPaymentIntentPreview(undefined);
    if (
      amountCents === undefined ||
      amountCents < 500 ||
      amountCents > 1_000_000
    ) {
      return;
    }
    (async () => {
      const openai = await openAi(session);
      const paymentIntent = (await openai.post(
        `${apiUrl}/stripe/payment_intent`,
        { body: { amount_cents: amountCents } }
      )) as PaymentIntentPreview;
      setPaymentIntentPreview(paymentIntent);
    })();
  }, [session, amountCents]);

  // confirmed
  const [confirming, setConfirming] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  // payment intent
  const [paymentIntent, setPaymentIntent] = useState<PaymentIntent>();

  // continue to checkout
  const handleConfirm = async () => {
    if (!paymentIntentPreview) return;
    setConfirming(true);
    const openai = await openAi(session);
    let paymentIntent: PaymentIntent;
    let attempt = 0;
    while (true) {
      try {
        paymentIntent = (await openai.post(`${apiUrl}/stripe/payment_intent`, {
          body: {
            amount_cents: amountCents,
            // receipt_email: profile.email,
            save_payment_method: true,
            confirm: true,
          },
        })) as PaymentIntent;
        break;
      } catch (error) {
        if (error instanceof APIError && error.status === 429 && attempt < 1) {
          attempt += 1;
          // sleep for 10 seconds
          await new Promise((resolve) => setTimeout(resolve, 10000));
        } else {
          throw error;
        }
      }
    }
    const customerSession = (await openai.get(
      `${apiUrl}/stripe/customer`
    )) as CustomerSession | null;
    if (customerSession === null) {
      onNeedBillingAddress();
    } else {
      setPaymentIntent(paymentIntent);
      setBillingAddress(customerSessionToBillingAddress(customerSession));
      setConfirming(false);
      setConfirmed(true);
    }
  };

  // purchasing
  const [purchasing, setPurchasing] = useState(false);

  // purchase
  const handlePurchase = async (stripe: Stripe, elements: StripeElements) => {
    if (paymentIntent === undefined) return;
    setPurchasing(true);
    const { selectedPaymentMethod, error: elementsError } =
      await elements.submit();
    if (elementsError) {
      console.error(elementsError);
      setPurchasing(false);
      return;
    }
    const { error: paymentError } = await stripe.confirmPayment({
      elements,
      clientSecret: paymentIntent.client_secret,
      confirmParams: {
        return_url: `${window.location.origin}/account/credits`,
        payment_method: selectedPaymentMethod,
      },
    });
    if (paymentError) {
      console.error(paymentError);
    }
    setPurchasing(false);
    onClose();
  };

  // submit
  const handleSubmit = (
    stripe: Stripe | null,
    elements: StripeElements | null
  ) => {
    if (confirmed) {
      if (!stripe || !elements) return;
      handlePurchase(stripe, elements);
    } else {
      handleConfirm();
    }
  };

  const [elementsReady, setElementsReady] = useState(false);

  if (customerSession === undefined) {
    return <LoadingDotsFull />;
  } else {
    return (
      <>
        <PurchaseCreditsAmount
          amount={amountCents}
          onChange={setAmountCents}
          confirmingOrConfirmed={confirming || confirmed}
        />
        <PurchaseCreditsBreakdown
          paymentIntentPreview={paymentIntentPreview}
          paymentIntent={paymentIntent}
          confirmingOrConfirmed={confirming || confirmed}
        />
        {confirmed && customerSession && paymentIntent && (
          <Elements
            key={`${confirmed}`}
            stripe={stripe}
            options={{
              appearance: { theme: theme === "light" ? "stripe" : "night" },
              customerSessionClientSecret: customerSession.client_secret,
              clientSecret: paymentIntent.client_secret,
            }}
          >
            <PaymentElement
              options={{ layout: "tabs" }}
              onReady={() => setElementsReady(true)}
            />
            {elementsReady && (
              <SubmitButtonInner
                onClick={handleSubmit}
                loading={purchasing || confirming || !paymentIntentPreview}
              >
                {confirmed ? "Purchase" : "Continue"}
              </SubmitButtonInner>
            )}
          </Elements>
        )}
        {!confirmed && (
          <SubmitButtonOuter
            onClick={() => handleSubmit(null, null)}
            loading={purchasing || confirming || !paymentIntentPreview}
          >
            {confirmed ? "Purchase" : "Continue"}
          </SubmitButtonOuter>
        )}
      </>
    );
  }
}

function PurchaseCreditsAmount({
  amount,
  onChange,
  confirmingOrConfirmed,
}: {
  amount?: number;
  onChange: (amount: number | undefined) => void;
  confirmingOrConfirmed?: boolean;
}): ReactElement {
  const [value, setValueState] = useState<string>(
    amount ? (amount / 100).toString() : ""
  );
  const [valueValid, setValueValid] = useState(true);
  const setValue = (newValue: string) => {
    const match = newValue.match(/^(\d*\.?\d{0,2})\d*$/)?.[1];
    const matchNumber =
      match && match !== "" && match !== "."
        ? Math.round(parseFloat(match) * 100)
        : undefined;
    if (matchNumber && matchNumber >= 500 && matchNumber <= 1_000_000) {
      setValueState(match!);
      setValueValid(true);
      onChange(matchNumber);
    } else {
      setValueState(newValue);
      setValueValid(false);
      onChange(undefined);
    }
  };
  return (
    <>
      <div
        className={cn(
          "flex",
          "border",
          "border-muted-secondary",
          "rounded-lg",
          "px-4"
        )}
      >
        <span
          className={cn("border-r", "pr-2", "border-muted-secondary", "py-2")}
        >
          Credits
        </span>
        <input
          type="number"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          className={cn(
            "flex-grow",
            "border-0",
            "outline-none",
            "[appearance:textfield]",
            "[&::-webkit-inner-spin-button]:appearance-none]",
            "[&::-webkit-outer-spin-button]:appearance-none]",
            "text-right",
            "font-mono",
            "text-lg",
            confirmingOrConfirmed && "cursor-text"
          )}
          disabled={confirmingOrConfirmed}
          readOnly={confirmingOrConfirmed}
        />
      </div>
      {!valueValid && (
        <div
          className={cn(
            "w-full",
            "inline-flex",
            "justify-center",
            "text-secondary"
          )}
        >
          Credits must be between 5 and 10000
        </div>
      )}
    </>
  );
}

function PurchaseCreditsBreakdown({
  paymentIntentPreview,
  paymentIntent,
  confirmingOrConfirmed,
  className,
}: {
  paymentIntentPreview?: PaymentIntentPreview | null;
  paymentIntent?: PaymentIntent | null;
  confirmingOrConfirmed?: boolean;
  className?: string;
}): ReactElement {
  const {
    credits,
    creditsZero,
    fee,
    feeZero,
    subtotal,
    subtotalZero,
    tax,
    taxZero,
    total,
    totalZero,
  } = useMemo(() => {
    const formatter = costFormatter();
    const creditsNumber = paymentIntent
      ? paymentIntent.credits_amount_cents / 100
      : paymentIntentPreview
      ? paymentIntentPreview.credits_amount_cents / 100
      : undefined;
    const feeNumber = paymentIntent
      ? paymentIntent.fee_amount_cents / 100
      : paymentIntentPreview
      ? paymentIntentPreview.fee_amount_cents / 100
      : undefined;
    const subtotalNumber = paymentIntent
      ? paymentIntent.subtotal_amount_cents / 100
      : paymentIntentPreview
      ? paymentIntentPreview.subtotal_amount_cents / 100
      : undefined;
    const taxNumber = paymentIntent
      ? paymentIntent.tax_amount_cents / 100
      : undefined;
    const totalNumber = paymentIntent
      ? paymentIntent.total_amount_cents / 100
      : undefined;
    return {
      credits:
        creditsNumber !== undefined
          ? formatter.format(creditsNumber)
          : undefined,
      creditsZero: creditsNumber === 0,
      fee: feeNumber !== undefined ? formatter.format(feeNumber) : undefined,
      feeZero: feeNumber === 0,
      subtotal:
        subtotalNumber !== undefined
          ? formatter.format(subtotalNumber)
          : undefined,
      subtotalZero: subtotalNumber === 0,
      tax: taxNumber !== undefined ? formatter.format(taxNumber) : undefined,
      taxZero: taxNumber === 0,
      total:
        totalNumber !== undefined ? formatter.format(totalNumber) : undefined,
      totalZero: totalNumber === 0,
    };
  }, [paymentIntent, paymentIntentPreview]);

  return (
    <div className={cn("space-y-1", "mx-2", className)}>
      <div
        className={cn(
          "flex",
          "justify-between",
          "items-center",
          creditsZero && "text-muted-primary"
        )}
      >
        <span className={cn("text-sm")}>Credits</span>
        <span className={cn("font-mono")}>
          {credits ? credits : <LoadingDots />}
        </span>
      </div>
      <div
        className={cn(
          "flex",
          "justify-between",
          "items-center",
          feeZero && "text-muted-primary"
        )}
      >
        <span className={cn("text-sm")}>Service fees</span>
        <span className={cn("font-mono")}>{fee ? fee : <LoadingDots />}</span>
      </div>
      {!confirmingOrConfirmed && (
        <div
          className={cn(
            "flex",
            "justify-between",
            "items-center",
            subtotalZero && "text-muted-primary"
          )}
        >
          <span className={cn("text-sm")}>Subtotal</span>
          <span className={cn("font-mono")}>
            {subtotal ? subtotal : <LoadingDots />}
          </span>
        </div>
      )}
      {confirmingOrConfirmed && (
        <>
          <div
            className={cn(
              "flex",
              "justify-between",
              "items-center",
              taxZero && "text-muted-primary"
            )}
          >
            <span className={cn("text-sm")}>Sales taxes</span>
            <span className={cn("font-mono")}>
              {tax ? tax : <LoadingDots />}
            </span>
          </div>
          <div
            className={cn(
              "flex",
              "justify-between",
              "items-center",
              totalZero && "text-muted-primary"
            )}
          >
            <span className={cn("text-sm")}>Total</span>
            <span className={cn("font-mono")}>
              {total ? total : <LoadingDots />}
            </span>
          </div>
        </>
      )}
    </div>
  );
}

function SubmitButtonInner({
  children,
  onClick,
  disabled,
  loading,
  className,
}: {
  children: ReactNode;
  onClick: (stripe: Stripe | null, elements: StripeElements | null) => void;
  disabled?: boolean;
  loading?: boolean;
  className?: string;
}) {
  const stripe = useStripe();
  const elements = useElements();
  return (
    <button
      onClick={() => onClick(stripe, elements)}
      disabled={loading}
      className={cn(
        "flex",
        "w-full",
        "items-center",
        "justify-center",
        "px-4",
        "py-2",
        "border",
        "rounded-lg",
        loading
          ? "cursor-wait"
          : disabled
          ? "cursor-not-allowed"
          : "cursor-pointer",
        "border-muted-secondary",
        "hover:border-highlight-secondary",
        "hover:bg-highlight-secondary",
        "transition-colors",
        "duration-200",
        className
      )}
    >
      {loading ? <LoadingDots /> : children}
    </button>
  );
}

function SubmitButtonOuter({
  children,
  onClick,
  disabled,
  loading,
  className,
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  loading?: boolean;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={loading}
      className={cn(
        "flex",
        "w-full",
        "items-center",
        "justify-center",
        "px-4",
        "py-2",
        "border",
        "rounded-lg",
        loading
          ? "cursor-wait"
          : disabled
          ? "cursor-not-allowed"
          : "cursor-pointer",
        "border-muted-secondary",
        "hover:border-highlight-secondary",
        "hover:bg-highlight-secondary",
        "transition-colors",
        "duration-200",
        className
      )}
    >
      {loading ? <LoadingDots /> : children}
    </button>
  );
}

function LoadingDotsFull({ className }: { className?: string }) {
  return (
    <div className={cn("flex", "w-full", "justify-center", className)}>
      <LoadingDots />
    </div>
  );
}

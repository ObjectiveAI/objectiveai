import { ReactElement } from "react";
import { SharedFooter } from "@/components/SharedFooter";
import { SharedHeader } from "@/components/SharedHeader";
import { Provider } from "@/provider";
import cn from "classnames";
import { MarkdownContent } from "../Markdown";

const content = `# Privacy Policy
**Last Updated: October 18th, 2025**

Objective Artificial Intelligence, Inc. (“ObjectiveAI,” “we,” “our,” or “us”) is committed to protecting your privacy. This Privacy Policy explains how we collect, use, disclose, and safeguard your information when you use our API and website (together, the “Service”). By using our Service, you agree to the practices described in this policy. If you do not agree, you may not use the Service.

---

## 1. Information We Collect
- **Account Data**: We do not store emails or passwords. We use external authentication providers and store only the user IDs from those tokens.
- **Billing Data**: Billing information is handled by Stripe. Stripe stores and processes this data; we do not retain it.
- **Usage Data**: We log all requests and responses to and from the API, including those made via the website client. Users may retrieve their own historical chats. We may display anonymized statistics (e.g., number of tokens, request counts) but never expose user inputs or outputs to other parties without user consent.
- **Technical Data**: We may collect device information, IP addresses, and log data to secure and operate the Service.
- **No Sensitive Data**: We do not collect government IDs, health data, or other special categories of personal information.

---

## 2. How We Use Information
We use collected data to:
- Provide and maintain the Service
- Secure the Service and authenticate access
- Monitor usage and generate anonymized statistics
- Improve models and services (we reserve this right)
- Comply with legal obligations

We do not share data for marketing purposes.

---

## 3. Sharing and Disclosure
- **Service Providers**: We use third-party providers (e.g., cloud infrastructure, upstream LLM APIs, Stripe) who may process limited data on our behalf.
- **Statistics**: We may share non-sensitive, aggregated usage statistics.
- **Legal Requirements**: We may disclose data if required by law or to protect rights, property, or safety.
- **No Sale of Data**: We do not sell personal data.

---

## 4. Data Security
We use authentication, encryption, and access controls to protect stored data. While no method of transmission or storage is fully secure, we take reasonable steps to safeguard your information.

---

## 5. Data Retention
- Usage data (including inputs/outputs and stats) may be retained indefinitely.
- Because we use external authentication, we do not maintain standalone user accounts and therefore do not currently provide account closure or deletion functionality.

---

## 6. Your Rights
Depending on your location, you may have rights under laws such as the GDPR and CCPA:
- Access your data
- Request correction of inaccurate data
- Request deletion of data (subject to limitations)
- Request a copy of your data in portable format
- Object to certain processing
- Restrict processing in certain circumstances
- Withdraw consent where applicable

To exercise these rights, email us at **admin@objective-ai.io**. We may require verification of your identity.

---

## 7. International Data Transfers
All data is stored and processed in the United States. By using the Service, you consent to this transfer and storage.

---

## 8. Eligibility
Our Service is intended for users 18 years of age and older.

---

## 9. Changes
We may update this Privacy Policy from time to time. Updates will be posted on this page with a new “Last Updated” date. Continued use of the Service means you accept the updated policy.

---

## 10. Contact
If you have questions about this Privacy Policy, email us at **admin@objective-ai.io**.`;

export function Privacy({
  session,
}: {
  session?: Provider.TokenSession;
}): ReactElement {
  return (
    <main className={cn("h-[100dvh]", "w-[100dvw]", "flex", "flex-col")}>
      <SharedHeader session={session} />
      <div className={cn("flex-grow", "overflow-auto")}>
        <div
          className={cn(
            "w-[calc(var(--spacing)*192)]",
            "max-w-[calc(100dvw-var(--spacing)*8)]",
            "mx-auto",
            "my-8"
          )}
        >
          <MarkdownContent content={content} />
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}

import { ReactElement } from "react";
import { SharedFooter } from "@/components/SharedFooter";
import { SharedHeader } from "@/components/SharedHeader";
import { Provider } from "@/provider";
import cn from "classnames";
import { MarkdownContent } from "../Markdown";

const content = `# Terms of Service
**Last Updated: October 18th, 2025**

Welcome to Objective Artificial Intelligence, Inc. (“ObjectiveAI,” “we,” “our,” or “us”). These Terms of Service (“Terms”) govern your access to and use of our API and website (together, the “Service”). Please read them carefully. By accessing or using the Service, you agree to be bound by these Terms and our Privacy Policy. If you do not agree, you may not use the Service.

---

## 1. Services Overview
ObjectiveAI provides access to artificial intelligence models through our API and website. We may update, modify, suspend, or discontinue any part of the Service at any time without liability.

---

## 2. Eligibility
You must be at least 18 years of age to use the Service. By using the Service, you represent that you meet this requirement and that your use of the Service complies with all applicable laws.

---

## 3. Accounts and Authentication
We use third-party authentication providers. We do not store emails or passwords. You are responsible for maintaining the security of your external authentication credentials. We are not responsible for any unauthorized access resulting from compromised credentials.

---

## 4. Credits and Payments
Access to the Service requires the purchase of credits.  
- **Purchases**: Credits are purchased through Stripe, our third-party payment processor.  
- **Refunds**: Refunds are available only within 24 hours of purchase and only if your account still has the purchased balance available. Partial refunds are not offered. To request a refund, contact us at **admin@objective-ai.io**.  
- **Expiration**: Credits do not currently expire.  
- **No Subscriptions**: We do not offer subscription plans at this time.  

---

## 5. User Content (Inputs and Outputs)
- **Ownership**: You retain ownership of all inputs and outputs you generate using the Service.  
- **Rights Reserved**: By using the Service, you grant ObjectiveAI the right to store and process your inputs and outputs in order to provide and improve the Service.  
- **Model Improvement**: We reserve the right to use stored data for model improvement.  
- **Upstream Providers**: Requests may be routed to upstream AI providers. By using the Service, you agree to the applicable terms of those providers. Users may configure providers that do not store data if they wish to avoid upstream storage.  

---

## 6. Acceptable Use
You may not use the Service for illegal, harmful, or fraudulent purposes. You are solely responsible for your activity when using the Service.

---

## 7. Service Availability
We strive to maintain reliable access but do not guarantee uptime. Service availability may depend on upstream providers. The Service is provided “as is” and “as available.”

---

## 8. Intellectual Property
ObjectiveAI retains all rights, title, and interest in and to its technology, branding, and services. Your use of the Service does not grant you any ownership of our intellectual property.

---

## 9. Disclaimers and Limitation of Liability
- **Disclaimers**: The Service is provided “as is” without warranties of any kind, express or implied, including fitness for a particular purpose or non-infringement.  
- **Liability Cap**: To the maximum extent permitted by law, ObjectiveAI’s total liability for any claim is limited to the greater of: (a) the amount you paid us in the 12 months prior to the event giving rise to the claim, or (b) $100.  

---

## 10. Termination
- **By You**: You may stop using the Service at any time.  
- **By Us**: We may suspend or terminate your access at any time, including for violations of these Terms. In such cases, we will refund your remaining purchased credit balance.  

---

## 11. Feedback
If you provide us with feedback, suggestions, or ideas, you grant us a perpetual, royalty-free right to use them without restriction or compensation.

---

## 12. Governing Law and Dispute Resolution
These Terms are governed by the laws of the State of Delaware. Any disputes will be resolved through **binding arbitration** on an individual basis. You waive the right to participate in class actions or class-wide arbitration.

---

## 13. Changes to These Terms
We may update these Terms from time to time. Updated versions will be posted on our website with a new “Last Updated” date. Continued use of the Service after updates constitutes acceptance of the revised Terms.

---

## 14. Contact Us
If you have questions about these Terms, please email us at **admin@objective-ai.io**.`;

export function Terms({
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

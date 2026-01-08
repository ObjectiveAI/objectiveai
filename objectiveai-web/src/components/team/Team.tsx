import { Provider } from "@/provider";
import { ReactElement } from "react";
import cn from "classnames";
import { SharedHeader } from "../SharedHeader";
import { SharedFooter } from "../SharedFooter";
import Image from "next/image";
import Link from "next/link";

export function Team({
  session,
}: {
  session?: Provider.TokenSession;
}): ReactElement {
  return (
    <main className={cn("h-[100dvh]", "w-[100dvw]", "flex", "flex-col")}>
      <SharedHeader session={session} />
      <div
        className={cn(
          "flex-grow",
          "items-center",
          "overflow-y-auto",
          "p-8",
          "space-y-8"
        )}
      >
        <div
          className={cn(
            "flex",
            "justify-center",
            "text-2xl",
            "lg:text-4xl",
            "mx-auto"
          )}
        >
          We&apos;re an AI Research Lab.
        </div>
        <div
          className={cn(
            "flex",
            "flex-wrap",
            "w-[calc(100%-var(--spacing)*4)]",
            "mx-auto",
            "gap-8",
            "justify-center"
          )}
        >
          <Profile
            name="Ronald Riggles"
            role="CEO"
            imageSrc="/ronald_riggles.jpg"
            description="Ronald's earliest memory is of a website. Devoted to Software, he believes that everything can be optimized. He studied Computer Science at Auburn University. His background includes Freelance Software Development and a tenure with the Alabama Department of Transportation. He takes a data-driven approach to leadership. He's been focused on AI ever since he realized just what all he can do with it."
          />
          <Profile
            name="Ryan Gibbs"
            role="Concept Developer"
            imageSrc="/ryan_gibbs.png"
            description="Ryan is a pioneer and explorer at heart. Well studied in physics, psychology, and the fundamental principles of nature, he sees AI as a key stepping stone in humanity's journey to unlocking its collective untapped potential energy. He synthesizes a scientific and common sense approach to technical and social problems alike, aiming to bring novel perspectives from abstract realms into the material world in ways that measurably improve productivity and quality of life. He is dedicated to manifesting the peak benefits of technology while preserving the core imperatives of the human spirit."
          />
          <Profile
            name="Maya Gore"
            role="Brand Designer"
            imageSrc="/maya_gore.jpg"
            description="Currently in her senior year to earn a BFA in Visual Arts with a concentration in Graphic Design, Maya is a student at Auburn University at Montgomery. Her background is in fundamental design principles, conceptual branding, and digital illustration."
          />
          <Profile
            name="Benjamin Taylor"
            role="Financial Advisor"
            imageSrc="/benjamin_taylor.png"
            description="Benjamin holds a degree in Applied Mathematics and brings over seven years of experience in corporate finance, along with hands-on involvement in marketing strategy. He combines analytical expertise with a passion for innovation, leveraging data-driven insights to enhance financial performance and strengthen market positioning. Fascinated by the rapid growth of artificial intelligence, he is particularly interested in how emerging technologies can transform financial modeling, strategic planning, customer engagement, and business development."
          />
        </div>
        <div
          className={cn(
            "flex",
            "justify-center",
            "text-2xl",
            "lg:text-4xl",
            "mx-auto"
          )}
        >
          Interested in building the future?&nbsp;
          <Link
            href="mailto:admin@objective-ai.io"
            className={cn(
              "text-primary",
              "hover:text-highlight-primary",
              "transition-colors",
              "duration-200",
              "underline"
            )}
          >
            Contact us
          </Link>
          .
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}

function Profile({
  name,
  role,
  imageSrc,
  description,
}: {
  name: string;
  role: string;
  imageSrc: string;
  description: string;
}): ReactElement {
  return (
    <div
      className={cn(
        "min-w-48",
        "w-[min(var(--spacing)*96,100%)]",
        "flex",
        "flex-col",
        "border",
        "border-muted-secondary",
        "rounded-lg",
        "p-8"
      )}
    >
      <div className={cn("relative", "w-full", "aspect-square", "self-center")}>
        <Image src={imageSrc} alt={name} fill />
      </div>
      <span className={cn("text-2xl", "font-light", "mt-2")}>{name}</span>
      <span className={cn("text-lg", "text-secondary", "font-bold", "mb-1")}>
        {role}
      </span>
      <p>{description}</p>
    </div>
  );
}

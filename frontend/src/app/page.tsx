import Image from "next/image";
import {SignIn} from "@/app/components/SignIn";

export default function Home() {
    return (
        <div className="max-w-7xl relative flex place-items-center">
            <div className="w-5xl relative">
                <div>
                    <h1 className="text-7xl font-bold text-left mb-10">
                        Your daily dose of code.
                    </h1>
                </div>
                <div className="mb-10">
                    <h2 className="text-3xl font-bold text-left mb-2">
                        😊 Meaningful commits
                    </h2>
                    <h2 className="text-3xl font-bold text-left mb-2">
                        ⚠️ Spontaneous merges
                    </h2>
                    <h2 className="text-3xl font-bold text-left mb-2">
                        🤳 Authentic pull requests
                    </h2>
                </div>
                <div>
                    <h3 className="text-xl text-left mb-10">
                        Everyday get a notification to share a genuine glimpse into your coding life with the people
                        you care most about.
                    </h3>
                </div>
                <div>
                    <SignIn/>
                </div>
            </div>
            <div className='w-5xl'>
                <Image
                    src="/snippet.svg"
                    alt="Vercel Logo"
                    width={1200}
                    height={500}
                    priority
                />
            </div>
        </div>
    );
}

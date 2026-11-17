#import <AppKit/AppKit.h>
#import <Sparkle/Sparkle.h>
// Sparkle is loaded only from this application's Frameworks, never from an
// environment-selected external dylib. Standalone cargo binaries fail closed.
@interface FNVUpdateOwner : NSObject <SPUUpdaterDelegate>
@property(nonatomic,strong) SPUStandardUpdaterController *controller;
@property(nonatomic,strong) NSMutableArray<NSNumber *> *events;
@property(nonatomic,copy) NSString *feed;
@property(nonatomic) BOOL detached;
@end
@implementation FNVUpdateOwner
- (NSString *)feedURLStringForUpdater:(SPUUpdater *)updater {return self.feed;}
- (void)push:(int)value {if (!self.detached && self.events.count<32) [self.events addObject:@(value)];}
- (void)updater:(SPUUpdater *)u didFindValidUpdate:(SUAppcastItem *)item {[self push:1];}
- (void)updaterDidNotFindUpdate:(SPUUpdater *)u {[self push:2];}
- (void)updaterDidNotFindUpdate:(SPUUpdater *)u error:(NSError *)error {[self push:2];}
- (void)updater:(SPUUpdater *)u didAbortWithError:(NSError *)error {[self push:3];}
- (void)updater:(SPUUpdater *)u willDownloadUpdate:(SUAppcastItem *)item withRequest:(NSMutableURLRequest *)r {[self push:4];}
- (void)updater:(SPUUpdater *)u didDownloadUpdate:(SUAppcastItem *)item {[self push:5];}
- (void)updater:(SPUUpdater *)u willInstallUpdate:(SUAppcastItem *)item {[self push:6];}
- (void)updater:(SPUUpdater *)u didFinishUpdateCycleForUpdateCheck:(SPUUpdateCheck)c error:(NSError *)e {[self push:7];}
@end
void *fnv_updater_create(const char *feed) {
    @autoreleasepool {
        if (![NSThread isMainThread]) return NULL;
        NSBundle *app=NSBundle.mainBundle;
        // Require an explicit Ed25519 trust anchor and disallow silent installs.
        NSString *key=[app objectForInfoDictionaryKey:@"SUPublicEDKey"];
        if (![app objectForInfoDictionaryKey:@"SUAllowsAutomaticUpdates"] ||
            ![app objectForInfoDictionaryKey:@"SUAutomaticallyUpdate"] ||
            ![app objectForInfoDictionaryKey:@"SUEnableAutomaticChecks"]) return NULL;
        if (!key || [[NSData alloc] initWithBase64EncodedString:key options:0].length!=32 ||
            [[app objectForInfoDictionaryKey:@"SUAllowsAutomaticUpdates"] boolValue] ||
            [[app objectForInfoDictionaryKey:@"SUAutomaticallyUpdate"] boolValue] ||
            [[app objectForInfoDictionaryKey:@"SUEnableAutomaticChecks"] boolValue]) return NULL;
        NSBundle *sparkle=[NSBundle bundleWithPath:[app.privateFrameworksPath stringByAppendingPathComponent:@"Sparkle.framework"]];
        if (![sparkle load]) return NULL;
        Class controllerClass=NSClassFromString(@"SPUStandardUpdaterController");
        if (!controllerClass) return NULL;
        FNVUpdateOwner *owner=[FNVUpdateOwner new];owner.events=[NSMutableArray new];owner.feed=@(feed);
        owner.controller=[[controllerClass alloc] initWithStartingUpdater:NO updaterDelegate:owner userDriverDelegate:nil];
        NSError *error=nil;
        if (![owner.controller.updater startUpdater:&error]) return NULL;
        return (__bridge_retained void *)owner;
    }
}
bool fnv_updater_check(void *handle, bool background) {
    FNVUpdateOwner *owner=(__bridge FNVUpdateOwner *)handle;
    if (![NSThread isMainThread] || owner.detached || !owner.controller.updater.canCheckForUpdates) return false;
    // Startup only probes availability; the Settings action opens the update UI.
    if (background) [owner.controller.updater checkForUpdateInformation];
    else [owner.controller checkForUpdates:nil];
    return true;
}
int fnv_updater_poll(void *handle) {
    FNVUpdateOwner *owner=(__bridge FNVUpdateOwner *)handle;
    if (!owner.events.count) return 0;
    int value=owner.events.firstObject.intValue;[owner.events removeObjectAtIndex:0];return value;
}
void fnv_updater_destroy(void *handle) {
    FNVUpdateOwner *owner=(__bridge_transfer FNVUpdateOwner *)handle;
    owner.detached=YES; // Sparkle weakly references the delegate; releasing owner detaches it.
    [owner.events removeAllObjects];owner.controller=nil;
}

#import <Foundation/Foundation.h>
#import "safeValueForKey.h"

// Helper to call `valueForKey` without swift crashing
id safeValueForKey(id object, NSString *key) {
  @try {
    return [object valueForKey:key];
  } @catch (NSException *exception) {
    return nil;
  }
}
